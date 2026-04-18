use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Utc;
use walkdir::WalkDir;

use crate::error::AppError;
use crate::models::{GodNode, NoteEntry, VaultIndex, VaultStats};
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

/// backlink 수 상위 N개 노트(= God Node) 반환.
/// - incoming link 기준, self-reference 제외, broken link 제외.
/// - 정렬: backlink_count desc → path asc (결정론적).
/// - backlink_count가 0인 노트는 포함하지 않음.
pub fn compute_top_god_nodes(index: &VaultIndex, limit: usize) -> Vec<GodNode> {
    if limit == 0 {
        return Vec::new();
    }

    let mut candidates: Vec<GodNode> = index
        .notes
        .iter()
        .filter_map(|note| {
            let sources = index.backlinks.get(&note.title)?;
            let count = sources.iter().filter(|src| **src != note.path).count();
            if count == 0 {
                return None;
            }
            Some(GodNode {
                path: note.path.clone(),
                title: note.title.clone(),
                note_type: note.frontmatter.as_ref().map(|fm| fm.note_type.clone()),
                backlink_count: count,
            })
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.backlink_count
            .cmp(&a.backlink_count)
            .then_with(|| a.path.cmp(&b.path))
    });
    candidates.truncate(limit);
    candidates
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

    // ── compute_top_god_nodes ──

    fn make_note(path: &str, title: &str, outgoing: Vec<&str>) -> NoteEntry {
        NoteEntry {
            path: path.into(),
            title: title.into(),
            frontmatter: None,
            outgoing_links: outgoing.into_iter().map(String::from).collect(),
            modified_at: 0,
            size: 0,
            body: String::new(),
        }
    }

    fn make_index(notes: Vec<NoteEntry>) -> VaultIndex {
        let backlinks = build_backlinks(&notes);
        VaultIndex {
            notes,
            backlinks,
            scanned_at: 0,
        }
    }

    #[test]
    fn top_god_nodes_empty_index_returns_empty() {
        let index = make_index(vec![]);
        assert_eq!(compute_top_god_nodes(&index, 5), vec![]);
    }

    #[test]
    fn top_god_nodes_zero_limit_returns_empty() {
        let index = make_index(vec![
            make_note("a.md", "a", vec!["b"]),
            make_note("b.md", "b", vec![]),
        ]);
        assert_eq!(compute_top_god_nodes(&index, 0), vec![]);
    }

    #[test]
    fn top_god_nodes_excludes_notes_without_backlinks() {
        // a→b, c: 아무도 참조 안 함
        let index = make_index(vec![
            make_note("a.md", "a", vec!["b"]),
            make_note("b.md", "b", vec![]),
            make_note("c.md", "c", vec![]),
        ]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "b");
        assert_eq!(result[0].backlink_count, 1);
    }

    #[test]
    fn top_god_nodes_orders_by_backlink_count_desc() {
        // b: 2 backlinks (a, c), e: 1 backlink (d)
        let index = make_index(vec![
            make_note("a.md", "a", vec!["b"]),
            make_note("b.md", "b", vec![]),
            make_note("c.md", "c", vec!["b"]),
            make_note("d.md", "d", vec!["e"]),
            make_note("e.md", "e", vec![]),
        ]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "b");
        assert_eq!(result[0].backlink_count, 2);
        assert_eq!(result[1].title, "e");
        assert_eq!(result[1].backlink_count, 1);
    }

    #[test]
    fn top_god_nodes_ties_broken_by_path_asc() {
        // a→x, a→y: x와 y 각각 1 backlink. path z.md < y.md 순으로 asc
        // x.path="z.md", y.path="y.md" → 기대: y가 먼저
        let index = make_index(vec![
            make_note("a.md", "a", vec!["x", "y"]),
            make_note("z.md", "x", vec![]),
            make_note("y.md", "y", vec![]),
        ]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "y.md");
        assert_eq!(result[1].path, "z.md");
    }

    #[test]
    fn top_god_nodes_excludes_self_reference() {
        // a→a (self) + b→a: a의 실제 backlink_count는 1
        let index = make_index(vec![
            make_note("a.md", "a", vec!["a"]),
            make_note("b.md", "b", vec!["a"]),
        ]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "a");
        assert_eq!(result[0].backlink_count, 1);
    }

    #[test]
    fn top_god_nodes_self_only_reference_returns_empty() {
        // a→a 만 존재: self 제외 후 0 → 결과 제외
        let index = make_index(vec![make_note("a.md", "a", vec!["a"])]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn top_god_nodes_excludes_broken_links() {
        // a→nonexistent (broken), a→b, c→b
        let index = make_index(vec![
            make_note("a.md", "a", vec!["nonexistent", "b"]),
            make_note("b.md", "b", vec![]),
            make_note("c.md", "c", vec!["b"]),
        ]);
        let result = compute_top_god_nodes(&index, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "b");
        assert_eq!(result[0].backlink_count, 2);
        // broken title "nonexistent"는 결과에 없어야 함
        assert!(result.iter().all(|g| g.title != "nonexistent"));
    }

    #[test]
    fn top_god_nodes_respects_limit() {
        // 3개 노트가 각각 backlink=1. limit=2면 2개만 반환, path asc.
        let index = make_index(vec![
            make_note("src.md", "src", vec!["a", "b", "c"]),
            make_note("a.md", "a", vec![]),
            make_note("b.md", "b", vec![]),
            make_note("c.md", "c", vec![]),
        ]);
        let result = compute_top_god_nodes(&index, 2);
        assert_eq!(result.len(), 2);
        // 동률이므로 path asc: a.md, b.md
        assert_eq!(result[0].path, "a.md");
        assert_eq!(result[1].path, "b.md");
    }
}
