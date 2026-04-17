use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Utc;
use walkdir::WalkDir;

use crate::error::AppError;
use crate::models::{NoteEntry, VaultIndex, VaultStats};
use crate::vault::parser;

/// 볼트 디렉토리를 재귀 스캔하여 인메모리 인덱스 구축
pub fn scan_vault(vault_path: &Path, exclude_dirs: &[String]) -> Result<VaultIndex, AppError> {
    if !vault_path.exists() {
        return Err(AppError::VaultNotFound(
            vault_path.to_string_lossy().to_string(),
        ));
    }

    let notes: Vec<NoteEntry> = WalkDir::new(vault_path)
        .into_iter()
        .filter_entry(|entry| {
            if !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            !exclude_dirs.iter().any(|ex| name == *ex)
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "md")
        })
        .filter_map(|e| parser::parse_note(e.path(), vault_path).ok())
        .collect();

    let backlinks = build_backlinks(&notes);

    Ok(VaultIndex {
        notes,
        backlinks,
        scanned_at: Utc::now().timestamp(),
    })
}

/// 노트 목록에서 역방향 링크 맵 구축 (target → [source paths])
pub fn build_backlinks(notes: &[NoteEntry]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();

    for note in notes {
        for link in &note.outgoing_links {
            map.entry(link.clone())
                .or_default()
                .insert(note.path.clone());
        }
    }

    map.into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

/// 어떤 노트에서도 참조되지 않는 고아 노트 목록 반환
pub fn find_orphan_notes(index: &VaultIndex) -> Vec<NoteEntry> {
    let referenced: HashSet<&str> = index.backlinks.keys().map(|k| k.as_str()).collect();
    index
        .notes
        .iter()
        .filter(|n| !referenced.contains(n.title.as_str()))
        .cloned()
        .collect()
}

/// 볼트 인덱스에서 통계 집계
pub fn compute_stats(index: &VaultIndex) -> VaultStats {
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_status: HashMap<String, usize> = HashMap::new();
    let mut by_folder: HashMap<String, usize> = HashMap::new();
    let mut all_tags: HashSet<String> = HashSet::new();
    let mut total_links: usize = 0;

    let note_titles: HashSet<&str> = index.notes.iter().map(|n| n.title.as_str()).collect();
    let referenced: HashSet<&str> = index.backlinks.keys().map(|k| k.as_str()).collect();

    for note in &index.notes {
        // type/status 집계
        if let Some(ref fm) = note.frontmatter {
            let type_key = serde_json::to_string(&fm.note_type)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            *by_type.entry(type_key).or_default() += 1;

            if let Some(ref status) = fm.status {
                let status_key = serde_json::to_string(status)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string();
                *by_status.entry(status_key).or_default() += 1;
            }

            for tag in &fm.tags {
                all_tags.insert(tag.clone());
            }
        }

        // 폴더 집계
        let folder = Path::new(&note.path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        let folder_key = if folder.is_empty() {
            ".".to_string()
        } else {
            folder
        };
        *by_folder.entry(folder_key).or_default() += 1;

        total_links += note.outgoing_links.len();
    }

    // orphan: 인덱스 내 어떤 노트에서도 참조되지 않는 노트
    let orphan_notes = index
        .notes
        .iter()
        .filter(|n| !referenced.contains(n.title.as_str()))
        .count();
    // Note: find_orphan_notes()를 호출하지 않는 이유 — referenced를 이미 계산했으므로 중복 방지

    // broken links: wikilink 대상 중 실제 노트가 없는 것
    let broken_links: Vec<String> = index
        .backlinks
        .keys()
        .filter(|target| !note_titles.contains(target.as_str()))
        .cloned()
        .collect();

    VaultStats {
        total_notes: index.notes.len(),
        by_type,
        by_status,
        by_folder,
        total_links,
        total_tags: all_tags.len(),
        orphan_notes,
        broken_links,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_vault_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_md(dir: &Path, rel_path: &str, content: &str) {
        let full = dir.join(rel_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }

    const TIL_NOTE: &str = "---\ntype: til\ncreated: 2026-04-16\ntags:\n  - rust\nstatus: seedling\n---\n# TIL\n[[note-b]]";
    const DECISION_NOTE: &str = "---\ntype: decision\ncreated: 2026-04-15\ntags:\n  - arch\nstatus: growing\n---\n# Decision\n[[note-b]]\n[[note-c]]";
    const PLAIN_NOTE: &str = "# Plain note\nNo frontmatter here";
    const INVALID_YAML_NOTE: &str = "---\ntype: [broken\n---\n# Bad YAML";

    // ── scan_vault ──

    #[test]
    fn scan_vault_returns_error_for_nonexistent_path() {
        let result = scan_vault(Path::new("/nonexistent/vault/path"), &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::VaultNotFound(_)));
    }

    #[test]
    fn scan_vault_returns_empty_index_for_empty_dir() {
        let dir = create_vault_dir();
        let index = scan_vault(dir.path(), &[]).unwrap();
        assert!(index.notes.is_empty());
        assert!(index.backlinks.is_empty());
        assert!(index.scanned_at > 0);
    }

    #[test]
    fn scan_vault_indexes_all_md_files() {
        let dir = create_vault_dir();
        write_md(dir.path(), "note-a.md", TIL_NOTE);
        write_md(dir.path(), "note-b.md", PLAIN_NOTE);
        write_md(dir.path(), "sub/note-c.md", DECISION_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        assert_eq!(index.notes.len(), 3);
        assert!(index.scanned_at > 0);
    }

    #[test]
    fn scan_vault_excludes_directories() {
        let dir = create_vault_dir();
        write_md(dir.path(), "note-a.md", TIL_NOTE);
        write_md(dir.path(), ".git/hooks/readme.md", PLAIN_NOTE);
        write_md(dir.path(), "dashboard/doc.md", PLAIN_NOTE);

        let exclude = vec![".git".to_string(), "dashboard".to_string()];
        let index = scan_vault(dir.path(), &exclude).unwrap();
        assert_eq!(index.notes.len(), 1);
        assert_eq!(index.notes[0].title, "note-a");
    }

    #[test]
    fn scan_vault_ignores_non_md_files() {
        let dir = create_vault_dir();
        write_md(dir.path(), "note.md", TIL_NOTE);
        write_md(dir.path(), "image.png", "not real png");
        write_md(dir.path(), "data.txt", "text file");

        let index = scan_vault(dir.path(), &[]).unwrap();
        assert_eq!(index.notes.len(), 1);
    }

    #[test]
    fn scan_vault_skips_parse_failures_gracefully() {
        let dir = create_vault_dir();
        write_md(dir.path(), "good-a.md", TIL_NOTE);
        write_md(dir.path(), "good-b.md", PLAIN_NOTE);
        // invalid YAML은 파싱 실패가 아니라 frontmatter=None으로 처리됨
        // 진짜 parse_note 실패를 유발하려면 파일 권한 제거 등이 필요하지만
        // 여기서는 모든 파일이 포함되는지 확인
        write_md(dir.path(), "bad.md", INVALID_YAML_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        // invalid YAML도 frontmatter=None으로 파싱 성공하므로 3개 모두 포함
        assert_eq!(index.notes.len(), 3);
    }

    #[test]
    fn scan_vault_builds_backlinks() {
        let dir = create_vault_dir();
        // note-a → [[note-b]]
        write_md(dir.path(), "note-a.md", TIL_NOTE);
        // note-b는 링크 없음
        write_md(dir.path(), "note-b.md", PLAIN_NOTE);
        // decision → [[note-b]], [[note-c]]
        write_md(dir.path(), "decision.md", DECISION_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        // note-b는 note-a와 decision에서 참조
        let note_b_backlinks = index.backlinks.get("note-b").unwrap();
        assert_eq!(note_b_backlinks.len(), 2);
        // note-c는 decision에서만 참조
        let note_c_backlinks = index.backlinks.get("note-c").unwrap();
        assert_eq!(note_c_backlinks.len(), 1);
    }

    // ── build_backlinks ──

    #[test]
    fn build_backlinks_maps_targets_to_sources() {
        let notes = vec![
            NoteEntry {
                path: "note-a.md".into(),
                title: "note-a".into(),
                frontmatter: None,
                outgoing_links: vec!["note-b".into()],
                modified_at: 0,
                size: 0,
                body: String::new(),
            },
            NoteEntry {
                path: "sub/note-c.md".into(),
                title: "note-c".into(),
                frontmatter: None,
                outgoing_links: vec!["note-b".into()],
                modified_at: 0,
                size: 0,
                body: String::new(),
            },
        ];

        let bl = build_backlinks(&notes);
        let sources = bl.get("note-b").unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&"note-a.md".to_string()));
        assert!(sources.contains(&"sub/note-c.md".to_string()));
    }

    #[test]
    fn build_backlinks_returns_empty_when_no_links() {
        let notes = vec![NoteEntry {
            path: "note.md".into(),
            title: "note".into(),
            frontmatter: None,
            outgoing_links: vec![],
            modified_at: 0,
            size: 0,
            body: String::new(),
        }];

        let bl = build_backlinks(&notes);
        assert!(bl.is_empty());
    }

    #[test]
    fn build_backlinks_deduplicates_sources() {
        // 같은 노트가 같은 대상을 두 번 참조
        let notes = vec![NoteEntry {
            path: "note-a.md".into(),
            title: "note-a".into(),
            frontmatter: None,
            outgoing_links: vec!["note-b".into(), "note-b".into()],
            modified_at: 0,
            size: 0,
            body: String::new(),
        }];

        let bl = build_backlinks(&notes);
        let sources = bl.get("note-b").unwrap();
        assert_eq!(sources.len(), 1);
    }

    // ── compute_stats ──

    #[test]
    fn compute_stats_counts_by_type_and_status() {
        let dir = create_vault_dir();
        write_md(dir.path(), "a.md", TIL_NOTE);
        write_md(dir.path(), "b.md", TIL_NOTE);
        write_md(dir.path(), "c.md", DECISION_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        let stats = compute_stats(&index);

        assert_eq!(stats.total_notes, 3);
        assert_eq!(*stats.by_type.get("til").unwrap(), 2);
        assert_eq!(*stats.by_type.get("decision").unwrap(), 1);
        assert_eq!(*stats.by_status.get("seedling").unwrap(), 2);
        assert_eq!(*stats.by_status.get("growing").unwrap(), 1);
    }

    #[test]
    fn compute_stats_counts_links_and_orphans() {
        let dir = create_vault_dir();
        // note-a → [[note-b]] (note-a는 아무도 참조 안 함 → orphan)
        write_md(dir.path(), "note-a.md", TIL_NOTE);
        // note-b는 링크 없음 (note-a가 참조 → orphan 아님)
        write_md(dir.path(), "note-b.md", PLAIN_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        let stats = compute_stats(&index);

        assert_eq!(stats.total_links, 1);
        assert_eq!(stats.orphan_notes, 1); // note-a
    }

    #[test]
    fn compute_stats_detects_broken_links() {
        let dir = create_vault_dir();
        // note-a → [[note-b]] (note-b 파일 없음)
        write_md(dir.path(), "note-a.md", TIL_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        let stats = compute_stats(&index);

        assert!(stats.broken_links.contains(&"note-b".to_string()));
    }

    #[test]
    fn compute_stats_counts_unique_tags() {
        let dir = create_vault_dir();
        // TIL_NOTE has tag "rust", DECISION_NOTE has tag "arch"
        write_md(dir.path(), "a.md", TIL_NOTE);
        write_md(dir.path(), "b.md", TIL_NOTE); // 같은 "rust" 태그
        write_md(dir.path(), "c.md", DECISION_NOTE); // "arch" 태그

        let index = scan_vault(dir.path(), &[]).unwrap();
        let stats = compute_stats(&index);

        assert_eq!(stats.total_tags, 2); // "rust", "arch"
    }

    #[test]
    fn compute_stats_counts_by_folder() {
        let dir = create_vault_dir();
        write_md(dir.path(), "dev/a.md", TIL_NOTE);
        write_md(dir.path(), "dev/b.md", TIL_NOTE);
        write_md(dir.path(), "decisions/c.md", DECISION_NOTE);
        write_md(dir.path(), "root-note.md", PLAIN_NOTE);

        let index = scan_vault(dir.path(), &[]).unwrap();
        let stats = compute_stats(&index);

        assert_eq!(*stats.by_folder.get("dev").unwrap(), 2);
        assert_eq!(*stats.by_folder.get("decisions").unwrap(), 1);
        // 루트의 노트는 "." 또는 "" 키로
        assert_eq!(stats.by_folder.values().sum::<usize>(), 4);
    }
}
