//! Project 레지스트리 IPC 커맨드.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState};
use crate::error::AppError;
use crate::models::{ProjectFileEntry, ProjectFolderNode};
use crate::notifications::NotificationsState;
use crate::project::{
    derive_project_id, derive_project_name, new_project_entry, normalize_root,
    read_last_graphify_at, ProjectEntry,
};
use crate::vault::parser::extract_frontmatter;
use crate::watcher::{self, WatcherState};

const TITLE_SCAN_BYTES: usize = 4096;
const DEFAULT_EXCLUDE_TREE_DIRS: &[&str] = &[".git", ".claude", "node_modules", "target"];

/// 폴더 등록 전 사전 점검 결과.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectInspection {
    pub root_path: String,
    pub root_exists: bool,
    pub docs_exists: bool,
    pub docs_is_dir: bool,
    pub graphify_out_exists: bool,
    pub already_registered: bool,
    pub suggested_name: String,
}

#[tauri::command]
pub fn inspect_project_root(
    config_state: State<'_, ConfigState>,
    root_path: String,
) -> Result<ProjectInspection, AppError> {
    let raw = PathBuf::from(&root_path);
    let normalized = normalize_root(&raw);
    let already_registered = {
        let config = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let id = derive_project_id(&normalized);
        config.projects.iter().any(|p| p.id == id)
    };
    Ok(inspect_path(&normalized, already_registered))
}

fn inspect_path(normalized: &Path, already_registered: bool) -> ProjectInspection {
    let root_exists = normalized.exists();
    let docs = normalized.join("docs");
    let docs_meta = fs::metadata(&docs).ok();
    let docs_exists = docs_meta.is_some();
    let docs_is_dir = docs_meta.map(|m| m.is_dir()).unwrap_or(false);
    let graphify_out_exists = normalized.join("graphify-out").is_dir();

    ProjectInspection {
        root_path: normalized.to_string_lossy().to_string(),
        root_exists,
        docs_exists,
        docs_is_dir,
        graphify_out_exists,
        already_registered,
        suggested_name: derive_project_name(normalized),
    }
}

#[tauri::command]
pub fn list_projects(
    config_state: State<'_, ConfigState>,
) -> Result<Vec<ProjectEntry>, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(config
        .projects
        .iter()
        .cloned()
        .map(refresh_last_graphify)
        .collect())
}

#[tauri::command]
pub fn get_active_project(
    config_state: State<'_, ConfigState>,
) -> Result<Option<ProjectEntry>, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(config.active_project().cloned().map(refresh_last_graphify))
}

#[tauri::command]
pub fn register_project(
    config_state: State<'_, ConfigState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    root_path: String,
    name: Option<String>,
    create_docs: bool,
) -> Result<ProjectEntry, AppError> {
    let raw = PathBuf::from(&root_path);
    if !raw.exists() {
        return Err(AppError::VaultNotFound(root_path));
    }
    let normalized = normalize_root(&raw);

    // docs/ 처리
    let docs_dir = normalized.join("docs");
    if !docs_dir.exists() {
        if create_docs {
            fs::create_dir_all(&docs_dir)?;
        } else {
            return Err(AppError::NoteNotFound(format!(
                "docs/ 가 존재하지 않음: {}",
                docs_dir.display()
            )));
        }
    } else if !docs_dir.is_dir() {
        return Err(AppError::InvalidPath(docs_dir.to_string_lossy().to_string()));
    }

    let new_id = derive_project_id(&normalized);

    let entry: ProjectEntry = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;

        if let Some(existing) = config.projects.iter().find(|p| p.id == new_id).cloned() {
            // 동일 root 재등록 → 활성만 전환
            config.active_project_id = Some(existing.id.clone());
            existing
        } else {
            let entry = new_project_entry(normalized.clone(), name);
            config.projects.push(entry.clone());
            config.active_project_id = Some(entry.id.clone());
            entry
        }
    };

    persist_and_restart_watcher(
        &config_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
    )?;

    Ok(refresh_last_graphify(entry))
}

#[tauri::command]
pub fn switch_project(
    config_state: State<'_, ConfigState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    id: String,
) -> Result<ProjectEntry, AppError> {
    let entry = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let Some(found) = config.projects.iter().find(|p| p.id == id).cloned() else {
            return Err(AppError::VaultNotFound(id));
        };
        config.active_project_id = Some(found.id.clone());
        found
    };

    persist_and_restart_watcher(
        &config_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
    )?;

    Ok(refresh_last_graphify(entry))
}

#[tauri::command]
pub fn remove_project(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    id: String,
) -> Result<Vec<ProjectEntry>, AppError> {
    let snapshot = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        if config.active_project_id.as_deref() == Some(id.as_str()) {
            return Err(AppError::VaultNotFound(
                "활성 프로젝트는 제거할 수 없음 (먼저 다른 프로젝트로 전환)".into(),
            ));
        }
        config.projects.retain(|p| p.id != id);
        config.clone()
    };

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    save_config(&snapshot, &app_data_dir)?;

    Ok(snapshot
        .projects
        .into_iter()
        .map(refresh_last_graphify)
        .collect())
}

#[tauri::command]
pub fn list_project_files(
    config_state: State<'_, ConfigState>,
    subpath: Option<String>,
) -> Result<Vec<ProjectFileEntry>, AppError> {
    let docs_path = {
        let config = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
        project.docs_path.clone()
    };

    let target = resolve_subpath(&docs_path, subpath.as_deref())?;
    if !target.is_dir() {
        return Ok(Vec::new());
    }

    list_files_in(&docs_path, &target)
}

#[tauri::command]
pub fn get_project_folder_tree(
    config_state: State<'_, ConfigState>,
) -> Result<ProjectFolderNode, AppError> {
    let (docs_path, exclude_dirs) = {
        let config = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
        (project.docs_path.clone(), config.exclude_dirs.clone())
    };

    if !docs_path.is_dir() {
        return Ok(ProjectFolderNode {
            name: "docs".to_string(),
            path: String::new(),
            note_count: 0,
            children: Vec::new(),
        });
    }

    let excluded: Vec<String> = exclude_dirs
        .into_iter()
        .chain(DEFAULT_EXCLUDE_TREE_DIRS.iter().map(|s| s.to_string()))
        .collect();

    Ok(build_folder_node(&docs_path, "docs", "", &excluded))
}

/// docs_path 기준의 안전한 절대 경로 변환. `..` 같은 traversal 차단.
fn resolve_subpath(docs_path: &Path, subpath: Option<&str>) -> Result<PathBuf, AppError> {
    let Some(sub) = subpath.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(docs_path.to_path_buf());
    };

    let candidate = Path::new(sub);
    if candidate.is_absolute() {
        return Err(AppError::InvalidPath(sub.to_string()));
    }
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => return Err(AppError::InvalidPath(sub.to_string())),
        }
    }

    Ok(docs_path.join(candidate))
}

fn list_files_in(docs_root: &Path, dir: &Path) -> Result<Vec<ProjectFileEntry>, AppError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "md" {
            continue;
        }
        let meta = entry.metadata()?;
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let (title, note_type) = read_title_and_type(&path);

        let relative = path
            .strip_prefix(docs_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        entries.push(ProjectFileEntry {
            path: relative,
            title,
            note_type,
            modified_at,
            size: meta.len(),
        });
    }
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(entries)
}

/// 파일의 첫 4 KiB 만 읽어 frontmatter 의 title/type 을 추출. 실패 시 파일 stem 사용.
fn read_title_and_type(path: &Path) -> (String, Option<String>) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    let Ok(content) = fs::read(path) else {
        return (stem, None);
    };
    let head_len = content.len().min(TITLE_SCAN_BYTES);
    let head = match std::str::from_utf8(&content[..head_len]) {
        Ok(s) => s,
        Err(_) => return (stem, None),
    };

    let Some(fm) = extract_frontmatter(head) else {
        return (stem, None);
    };

    let title = fm
        .extra
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or(stem);
    let note_type = serde_json::to_value(&fm.note_type)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    (title, note_type)
}

fn build_folder_node(
    abs_path: &Path,
    name: &str,
    rel_path: &str,
    exclude_dirs: &[String],
) -> ProjectFolderNode {
    let mut note_count = 0usize;
    let mut child_nodes: Vec<ProjectFolderNode> = Vec::new();

    let Ok(read) = fs::read_dir(abs_path) else {
        return ProjectFolderNode {
            name: name.to_string(),
            path: rel_path.to_string(),
            note_count: 0,
            children: Vec::new(),
        };
    };

    for entry in read.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if path.is_dir() {
            if file_name.starts_with('.') {
                continue;
            }
            if exclude_dirs.iter().any(|d| d == file_name) {
                continue;
            }
            let child_rel = if rel_path.is_empty() {
                file_name.to_string()
            } else {
                format!("{}/{}", rel_path, file_name)
            };
            child_nodes.push(build_folder_node(&path, file_name, &child_rel, exclude_dirs));
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if ext == "md" {
                note_count += 1;
            }
        }
    }

    child_nodes.sort_by(|a, b| a.name.cmp(&b.name));

    ProjectFolderNode {
        name: name.to_string(),
        path: rel_path.to_string(),
        note_count,
        children: child_nodes,
    }
}

/// 호출 시점의 graph.json mtime 으로 last_graphify_at 재계산.
fn refresh_last_graphify(mut entry: ProjectEntry) -> ProjectEntry {
    entry.last_graphify_at = read_last_graphify_at(&entry.graphify_out_path);
    entry
}

/// config 저장 + 활성 프로젝트 root 기준으로 watcher 재시작.
/// 활성 프로젝트가 없으면 watcher 미기동.
fn persist_and_restart_watcher(
    config_state: &ConfigState,
    watcher_state: &WatcherState,
    notifications_state: &NotificationsState,
    app_handle: &AppHandle,
) -> Result<(), AppError> {
    let config_snapshot = {
        let guard = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        guard.clone()
    };
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    save_config(&config_snapshot, &app_data_dir)?;

    let watch_root = config_snapshot.active_project().map(|p| p.root_path.clone());

    {
        let mut guard = watcher_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = None; // 기존 watcher drop
    }

    if let Some(root) = watch_root {
        let new_watcher = watcher::start_watching(
            app_handle.clone(),
            &root,
            &config_snapshot.exclude_dirs,
            config_snapshot.watch_debounce_ms,
            (*notifications_state).clone(),
        )?;
        let mut guard = watcher_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = Some(new_watcher);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use tempfile::TempDir;

    // --- inspect_path ---

    #[test]
    fn inspect_missing_root() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("ghost");
        let r = inspect_path(&nonexistent, false);
        assert!(!r.root_exists);
        assert!(!r.docs_exists);
        assert!(!r.graphify_out_exists);
        assert!(!r.already_registered);
        assert_eq!(r.suggested_name, "ghost");
    }

    #[test]
    fn inspect_root_without_docs_or_graphify() {
        let dir = TempDir::new().unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(!r.docs_exists);
        assert!(!r.docs_is_dir);
        assert!(!r.graphify_out_exists);
    }

    #[test]
    fn inspect_root_with_docs_as_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("docs"), "not a dir").unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(r.docs_exists);
        assert!(!r.docs_is_dir);
    }

    #[test]
    fn inspect_root_with_docs_and_graphify() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("graphify-out")).unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(r.docs_exists);
        assert!(r.docs_is_dir);
        assert!(r.graphify_out_exists);
    }

    #[test]
    fn inspect_already_registered_flag_pass_through() {
        let dir = TempDir::new().unwrap();
        let r = inspect_path(dir.path(), true);
        assert!(r.already_registered);
    }

    #[test]
    fn inspect_suggested_name_uses_basename() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("my-proj");
        fs::create_dir_all(&sub).unwrap();
        let r = inspect_path(&sub, false);
        assert_eq!(r.suggested_name, "my-proj");
    }

    #[test]
    fn refresh_sets_last_graphify_when_present() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("graphify-out")).unwrap();
        fs::write(dir.path().join("graphify-out/graph.json"), "{}").unwrap();

        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let refreshed = refresh_last_graphify(entry);
        assert!(refreshed.last_graphify_at.is_some());
    }

    #[test]
    fn refresh_keeps_none_when_absent() {
        let dir = TempDir::new().unwrap();
        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let refreshed = refresh_last_graphify(entry);
        assert!(refreshed.last_graphify_at.is_none());
    }

    // --- resolve_subpath ---

    #[test]
    fn resolve_subpath_none_returns_root() {
        let dir = TempDir::new().unwrap();
        let r = resolve_subpath(dir.path(), None).unwrap();
        assert_eq!(r, dir.path());
    }

    #[test]
    fn resolve_subpath_blank_returns_root() {
        let dir = TempDir::new().unwrap();
        let r = resolve_subpath(dir.path(), Some("  ")).unwrap();
        assert_eq!(r, dir.path());
    }

    #[test]
    fn resolve_subpath_normal_segments() {
        let dir = TempDir::new().unwrap();
        let r = resolve_subpath(dir.path(), Some("decisions/2026")).unwrap();
        assert_eq!(r, dir.path().join("decisions").join("2026"));
    }

    #[test]
    fn resolve_subpath_rejects_dotdot() {
        let dir = TempDir::new().unwrap();
        let err = resolve_subpath(dir.path(), Some("../etc"));
        assert!(matches!(err, Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn resolve_subpath_rejects_absolute() {
        let dir = TempDir::new().unwrap();
        let err = resolve_subpath(dir.path(), Some("/etc"));
        assert!(matches!(err, Err(AppError::InvalidPath(_))));
    }

    // --- list_files_in ---

    fn write_md(dir: &Path, name: &str, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn list_files_only_md_at_top_level() {
        let dir = TempDir::new().unwrap();
        write_md(dir.path(), "a.md", "# A");
        write_md(dir.path(), "b.txt", "ignored");
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        write_md(&dir.path().join("sub"), "c.md", "# C"); // 재귀 X

        let entries = list_files_in(dir.path(), dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "a.md");
    }

    #[test]
    fn list_files_subdir_relative_path() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("decisions");
        write_md(&sub, "2026.md", "# 2026");

        let entries = list_files_in(dir.path(), &sub).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "decisions/2026.md");
    }

    #[test]
    fn list_files_sorted_by_modified_desc() {
        use std::thread::sleep;
        use std::time::Duration;
        let dir = TempDir::new().unwrap();
        write_md(dir.path(), "old.md", "# old");
        sleep(Duration::from_millis(1100));
        write_md(dir.path(), "new.md", "# new");

        let entries = list_files_in(dir.path(), dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "new.md");
        assert_eq!(entries[1].path, "old.md");
    }

    // --- read_title_and_type ---

    #[test]
    fn read_title_falls_back_to_stem_without_frontmatter() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("plain-note.md");
        fs::write(&p, "# Heading\nbody").unwrap();
        let (title, note_type) = read_title_and_type(&p);
        assert_eq!(title, "plain-note");
        assert!(note_type.is_none());
    }

    #[test]
    fn read_title_uses_frontmatter_title_when_present() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("note.md");
        fs::write(
            &p,
            "---\ntype: decision\ncreated: 2026-04-26\ntitle: My Decision\n---\n# Body",
        )
        .unwrap();
        let (title, note_type) = read_title_and_type(&p);
        assert_eq!(title, "My Decision");
        assert_eq!(note_type.as_deref(), Some("decision"));
    }

    #[test]
    fn read_title_extracts_type_without_explicit_title() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("note.md");
        fs::write(&p, "---\ntype: til\ncreated: 2026-04-26\n---\nbody").unwrap();
        let (title, note_type) = read_title_and_type(&p);
        assert_eq!(title, "note");
        assert_eq!(note_type.as_deref(), Some("til"));
    }

    // --- build_folder_node ---

    #[test]
    fn folder_tree_empty_dir() {
        let dir = TempDir::new().unwrap();
        let tree = build_folder_node(dir.path(), "docs", "", &[]);
        assert_eq!(tree.name, "docs");
        assert_eq!(tree.path, "");
        assert_eq!(tree.note_count, 0);
        assert!(tree.children.is_empty());
    }

    #[test]
    fn folder_tree_counts_direct_md_only() {
        let dir = TempDir::new().unwrap();
        write_md(dir.path(), "a.md", "");
        write_md(dir.path(), "b.md", "");
        write_md(dir.path(), "c.txt", "");
        write_md(&dir.path().join("sub"), "d.md", "");

        let tree = build_folder_node(dir.path(), "docs", "", &[]);
        assert_eq!(tree.note_count, 2);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "sub");
        assert_eq!(tree.children[0].note_count, 1);
        assert_eq!(tree.children[0].path, "sub");
    }

    #[test]
    fn folder_tree_skips_dotfile_and_excluded_dirs() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        write_md(&dir.path().join("real"), "a.md", "");

        let excluded = vec!["node_modules".to_string()];
        let tree = build_folder_node(dir.path(), "docs", "", &excluded);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "real");
    }

    #[test]
    fn folder_tree_children_alphabetical() {
        let dir = TempDir::new().unwrap();
        for n in &["zeta", "alpha", "mu"] {
            fs::create_dir_all(dir.path().join(n)).unwrap();
        }
        let tree = build_folder_node(dir.path(), "docs", "", &[]);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn folder_tree_nested_paths() {
        let dir = TempDir::new().unwrap();
        write_md(&dir.path().join("a/b/c"), "leaf.md", "");
        let tree = build_folder_node(dir.path(), "docs", "", &[]);
        let a = &tree.children[0];
        let b = &a.children[0];
        let c = &b.children[0];
        assert_eq!(a.path, "a");
        assert_eq!(b.path, "a/b");
        assert_eq!(c.path, "a/b/c");
        assert_eq!(c.note_count, 1);
    }

    #[test]
    fn active_project_resolution_via_config() {
        let dir = TempDir::new().unwrap();
        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let id = entry.id.clone();
        let config = AppConfig {
            projects: vec![entry.clone()],
            active_project_id: Some(id.clone()),
            ..Default::default()
        };
        assert_eq!(config.active_project().map(|p| p.id.clone()), Some(id));
    }
}
