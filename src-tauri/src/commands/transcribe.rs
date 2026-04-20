use std::fs;
use std::path::{Path, PathBuf};

use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;

const ALLOWED_EXTENSIONS: &[&str] = &["m4a", "webm", "ogg"];
const RECORDINGS_SUBDIR: &str = "_sources/recordings";

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
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
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
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    delete_recording_impl(vault_path, &filename)
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
}
