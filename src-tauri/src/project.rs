//! Project 레지스트리 관련 순수 헬퍼.
//!
//! - normalize_root: 경로 정규화 (trailing slash 제거 + 절대 경로 변환)
//! - derive_project_id: blake3(정규화 경로) → 16자 hex
//! - derive_project_name: 폴더명 추출, 실패 시 "project" fallback
//! - read_last_graphify_at: graphify-out/graph.json 의 mtime 을 RFC3339로
//! - new_project_entry: 새 ProjectEntry 생성

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 등록된 프로젝트 항목.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,
    pub docs_path: PathBuf,
    pub graphify_out_path: PathBuf,
    pub registered_at: String,
    pub last_graphify_at: Option<String>,
}

/// 경로 정규화: trailing slash 제거 + 상대경로는 cwd 기준 절대 경로로 변환.
/// canonicalize 는 하지 않음 (심볼릭 링크 보존).
pub fn normalize_root(input: &Path) -> PathBuf {
    let s = input.to_string_lossy();
    let trimmed = s.trim_end_matches('/');
    let as_path = if trimmed.is_empty() {
        PathBuf::from("/")
    } else {
        PathBuf::from(trimmed)
    };

    if as_path.is_absolute() {
        as_path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&as_path))
            .unwrap_or(as_path)
    }
}

/// 정규화된 경로 → blake3 해시의 처음 16자 hex.
/// 동일 경로 → 동일 id (deterministic).
pub fn derive_project_id(root_path: &Path) -> String {
    let bytes = root_path.to_string_lossy();
    let hash = blake3::hash(bytes.as_bytes());
    hash.to_hex()[..16].to_string()
}

/// 폴더명에서 표시용 이름을 추출. 실패 시 "project" fallback.
pub fn derive_project_name(root_path: &Path) -> String {
    root_path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| "project".to_string())
}

/// graphify-out/graph.json 의 mtime 을 RFC3339 문자열로. 없으면 None.
pub fn read_last_graphify_at(graphify_out_path: &Path) -> Option<String> {
    let graph_json = graphify_out_path.join("graph.json");
    let metadata = fs::metadata(&graph_json).ok()?;
    let mtime: SystemTime = metadata.modified().ok()?;
    let dt: DateTime<Utc> = mtime.into();
    Some(dt.to_rfc3339())
}

/// 새 ProjectEntry 생성. id, registered_at, last_graphify_at 자동 채움.
/// root_path 는 호출 전 normalize_root 를 거쳤다고 가정.
pub fn new_project_entry(root_path: PathBuf, name: Option<String>) -> ProjectEntry {
    let id = derive_project_id(&root_path);
    let resolved_name = name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| derive_project_name(&root_path));
    let docs_path = root_path.join("docs");
    let graphify_out_path = root_path.join("graphify-out");
    let last_graphify_at = read_last_graphify_at(&graphify_out_path);
    let registered_at = Utc::now().to_rfc3339();

    ProjectEntry {
        id,
        name: resolved_name,
        root_path,
        docs_path,
        graphify_out_path,
        registered_at,
        last_graphify_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- normalize_root ---

    #[test]
    fn normalize_strips_trailing_slash() {
        assert_eq!(
            normalize_root(Path::new("/foo/bar/")),
            PathBuf::from("/foo/bar")
        );
    }

    #[test]
    fn normalize_already_canonical() {
        assert_eq!(
            normalize_root(Path::new("/foo/bar")),
            PathBuf::from("/foo/bar")
        );
    }

    #[test]
    fn normalize_relative_to_absolute() {
        let result = normalize_root(Path::new("./project"));
        assert!(result.is_absolute());
        assert!(result.ends_with("project"));
    }

    // --- derive_project_id ---

    #[test]
    fn derive_id_is_deterministic() {
        let path = Path::new("/Users/uno/work/my-project");
        assert_eq!(derive_project_id(path), derive_project_id(path));
    }

    #[test]
    fn derive_id_returns_16_hex_chars() {
        let id = derive_project_id(Path::new("/some/path"));
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn derive_id_distinct_for_distinct_paths() {
        let a = derive_project_id(Path::new("/a"));
        let b = derive_project_id(Path::new("/b"));
        assert_ne!(a, b);
    }

    // --- derive_project_name ---

    #[test]
    fn derive_name_from_basename() {
        assert_eq!(
            derive_project_name(Path::new("/foo/my-project")),
            "my-project"
        );
    }

    #[test]
    fn derive_name_root_fallback() {
        assert_eq!(derive_project_name(Path::new("/")), "project");
    }

    // --- read_last_graphify_at ---

    #[test]
    fn read_graphify_at_returns_none_when_dir_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(read_last_graphify_at(&missing).is_none());
    }

    #[test]
    fn read_graphify_at_returns_none_when_graph_json_missing() {
        let dir = TempDir::new().unwrap();
        // graphify-out 디렉토리는 있지만 graph.json 없음
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        assert!(read_last_graphify_at(&out).is_none());
    }

    #[test]
    fn read_graphify_at_returns_rfc3339_when_present() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("graph.json"), "{}").unwrap();

        let result = read_last_graphify_at(&out).expect("should read mtime");
        // RFC3339 파싱 가능해야 함
        DateTime::parse_from_rfc3339(&result).expect("valid rfc3339");
    }

    // --- new_project_entry ---

    #[test]
    fn new_entry_uses_basename_when_name_omitted() {
        let entry = new_project_entry(PathBuf::from("/Users/uno/work/foo"), None);
        assert_eq!(entry.name, "foo");
    }

    #[test]
    fn new_entry_uses_provided_name_when_given() {
        let entry = new_project_entry(
            PathBuf::from("/Users/uno/work/foo"),
            Some("My Project".to_string()),
        );
        assert_eq!(entry.name, "My Project");
    }

    #[test]
    fn new_entry_falls_back_when_name_is_blank() {
        let entry = new_project_entry(
            PathBuf::from("/Users/uno/work/foo"),
            Some("   ".to_string()),
        );
        assert_eq!(entry.name, "foo");
    }

    #[test]
    fn new_entry_derives_paths() {
        let entry = new_project_entry(PathBuf::from("/p"), None);
        assert_eq!(entry.docs_path, PathBuf::from("/p/docs"));
        assert_eq!(entry.graphify_out_path, PathBuf::from("/p/graphify-out"));
    }

    #[test]
    fn new_entry_id_matches_derive() {
        let path = PathBuf::from("/some/proj");
        let entry = new_project_entry(path.clone(), None);
        assert_eq!(entry.id, derive_project_id(&path));
    }

    #[test]
    fn new_entry_registered_at_is_rfc3339() {
        let entry = new_project_entry(PathBuf::from("/p"), None);
        DateTime::parse_from_rfc3339(&entry.registered_at).expect("valid rfc3339");
    }

    #[test]
    fn new_entry_last_graphify_none_when_no_graph_json() {
        let dir = TempDir::new().unwrap();
        let entry = new_project_entry(dir.path().to_path_buf(), None);
        assert!(entry.last_graphify_at.is_none());
    }

    #[test]
    fn new_entry_last_graphify_some_when_graph_json_exists() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("graph.json"), "{}").unwrap();

        let entry = new_project_entry(dir.path().to_path_buf(), None);
        assert!(entry.last_graphify_at.is_some());
    }
}
