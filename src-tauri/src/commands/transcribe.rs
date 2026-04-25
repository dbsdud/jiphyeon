use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;

const ALLOWED_EXTENSIONS: &[&str] = &["m4a", "webm", "ogg", "wav", "mp3"];
const RECORDINGS_SUBDIR: &str = "_sources/recordings";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordingEntry {
    pub filename: String,
    pub size: u64,
    pub modified_at: i64,
}

/// 파일명 유효성 검증. 상위 경로·구분자·잘못된 확장자를 거부.
fn validate_filename(filename: &str) -> Result<(), AppError> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return Err(AppError::InvalidPath(filename.to_string()));
    }
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::InvalidExtension(ext));
    }
    Ok(())
}

/// 녹음 바이너리를 `vault_path/_sources/recordings/` 하위에 저장.
/// 반환: 저장된 절대 경로.
pub fn save_recording_impl(
    vault_path: &Path,
    filename: &str,
    bytes: &[u8],
) -> Result<PathBuf, AppError> {
    validate_filename(filename)?;
    if bytes.is_empty() {
        return Err(AppError::EmptyRecording);
    }
    let dir = vault_path.join(RECORDINGS_SUBDIR);
    fs::create_dir_all(&dir)?;
    let target = dir.join(filename);
    if target.exists() {
        return Err(AppError::FileExists(filename.to_string()));
    }
    fs::write(&target, bytes)?;
    Ok(target)
}

/// `_sources/recordings/`의 오디오 파일을 나열. modified_at 내림차순.
pub fn list_recordings_impl(vault_path: &Path) -> Result<Vec<RecordingEntry>, AppError> {
    let rec_dir = vault_path.join(RECORDINGS_SUBDIR);
    if !rec_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<RecordingEntry> = Vec::new();
    for entry in fs::read_dir(&rec_dir)? {
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
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let Some(filename) = path.file_name().and_then(|n| n.to_str()).map(String::from)
        else {
            continue;
        };
        let meta = entry.metadata()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        entries.push(RecordingEntry {
            filename,
            size: meta.len(),
            modified_at: modified,
        });
    }
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(entries)
}

/// `_sources/recordings/<filename>` 삭제. 파일명 유효성 재검증으로 path traversal 방지.
pub fn delete_recording_impl(vault_path: &Path, filename: &str) -> Result<(), AppError> {
    validate_filename(filename)?;
    let target = vault_path.join(RECORDINGS_SUBDIR).join(filename);
    if !target.exists() {
        return Err(AppError::NoteNotFound(filename.to_string()));
    }
    fs::remove_file(&target)?;
    Ok(())
}

#[tauri::command]
pub fn save_recording(
    config_state: State<'_, ConfigState>,
    filename: String,
    bytes: Vec<u8>,
) -> Result<String, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    let vault_path = &project.docs_path;
    let path = save_recording_impl(vault_path, &filename, &bytes)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_recording(
    config_state: State<'_, ConfigState>,
    filename: String,
) -> Result<(), AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    let vault_path = &project.docs_path;
    delete_recording_impl(vault_path, &filename)
}

#[tauri::command]
pub fn list_recordings(
    config_state: State<'_, ConfigState>,
) -> Result<Vec<RecordingEntry>, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    let vault_path = &project.docs_path;
    list_recordings_impl(vault_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_bytes() -> Vec<u8> {
        vec![0u8; 64]
    }

    #[test]
    fn save_recording_creates_recordings_dir_if_missing() {
        let dir = tempdir().unwrap();
        let result = save_recording_impl(dir.path(), "take.m4a", &sample_bytes());
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert!(dir.path().join(RECORDINGS_SUBDIR).is_dir());
    }

    #[test]
    fn save_recording_writes_bytes_to_expected_path() {
        let dir = tempdir().unwrap();
        let bytes = vec![1, 2, 3, 4];
        let path = save_recording_impl(dir.path(), "take.webm", &bytes).unwrap();
        assert_eq!(path, dir.path().join(RECORDINGS_SUBDIR).join("take.webm"));
        let written = fs::read(&path).unwrap();
        assert_eq!(written, bytes);
    }

    #[test]
    fn save_recording_rejects_path_traversal() {
        let dir = tempdir().unwrap();
        let dotdot = save_recording_impl(dir.path(), "../evil.m4a", &sample_bytes());
        assert!(matches!(dotdot, Err(AppError::InvalidPath(_))));
        let slash = save_recording_impl(dir.path(), "sub/evil.m4a", &sample_bytes());
        assert!(matches!(slash, Err(AppError::InvalidPath(_))));
        let backslash = save_recording_impl(dir.path(), "sub\\evil.m4a", &sample_bytes());
        assert!(matches!(backslash, Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn save_recording_rejects_unknown_extension() {
        let dir = tempdir().unwrap();
        let result = save_recording_impl(dir.path(), "bad.exe", &sample_bytes());
        assert!(matches!(result, Err(AppError::InvalidExtension(_))));
    }

    #[test]
    fn save_recording_rejects_empty_bytes() {
        let dir = tempdir().unwrap();
        let result = save_recording_impl(dir.path(), "empty.ogg", &[]);
        assert!(matches!(result, Err(AppError::EmptyRecording)));
    }

    #[test]
    fn save_recording_errors_when_file_exists() {
        let dir = tempdir().unwrap();
        let bytes = sample_bytes();
        save_recording_impl(dir.path(), "dup.m4a", &bytes).unwrap();
        let second = save_recording_impl(dir.path(), "dup.m4a", &bytes);
        assert!(matches!(second, Err(AppError::FileExists(_))));
    }

    #[test]
    fn save_recording_accepts_all_allowed_extensions() {
        let dir = tempdir().unwrap();
        for (i, ext) in ["m4a", "webm", "ogg"].iter().enumerate() {
            let name = format!("ok-{i}.{ext}");
            let result = save_recording_impl(dir.path(), &name, &sample_bytes());
            assert!(result.is_ok(), "ext {ext} should be allowed, got {result:?}");
        }
    }

    #[test]
    fn delete_recording_removes_existing_file() {
        let dir = tempdir().unwrap();
        let saved = save_recording_impl(dir.path(), "to-delete.m4a", &sample_bytes()).unwrap();
        assert!(saved.exists());
        delete_recording_impl(dir.path(), "to-delete.m4a").unwrap();
        assert!(!saved.exists());
    }

    #[test]
    fn delete_recording_rejects_path_traversal() {
        let dir = tempdir().unwrap();
        let result = delete_recording_impl(dir.path(), "../evil.m4a");
        assert!(matches!(result, Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn delete_recording_errors_when_file_missing() {
        let dir = tempdir().unwrap();
        let result = delete_recording_impl(dir.path(), "ghost.m4a");
        assert!(matches!(result, Err(AppError::NoteNotFound(_))));
    }

    #[test]
    fn list_recordings_returns_empty_when_dir_absent() {
        let dir = tempdir().unwrap();
        let result = list_recordings_impl(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_recordings_returns_empty_for_empty_dir() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(RECORDINGS_SUBDIR)).unwrap();
        let result = list_recordings_impl(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_recordings_returns_entry_for_audio_file() {
        let dir = tempdir().unwrap();
        save_recording_impl(dir.path(), "a.m4a", &sample_bytes()).unwrap();
        let result = list_recordings_impl(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filename, "a.m4a");
        assert!(result[0].size > 0);
    }

    #[test]
    fn list_recordings_ignores_non_audio_files() {
        let dir = tempdir().unwrap();
        let rec_dir = dir.path().join(RECORDINGS_SUBDIR);
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("a.m4a"), b"audio").unwrap();
        fs::write(rec_dir.join("note.md"), b"text").unwrap();
        fs::write(rec_dir.join(".DS_Store"), b"meta").unwrap();
        let result = list_recordings_impl(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filename, "a.m4a");
    }

    #[test]
    fn list_recordings_sorted_by_modified_desc() {
        use std::thread::sleep;
        use std::time::Duration;
        let dir = tempdir().unwrap();
        save_recording_impl(dir.path(), "old.m4a", &sample_bytes()).unwrap();
        sleep(Duration::from_millis(1100));
        save_recording_impl(dir.path(), "new.m4a", &sample_bytes()).unwrap();
        let result = list_recordings_impl(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].filename, "new.m4a");
        assert_eq!(result[1].filename, "old.m4a");
    }
}
