use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::AppError;
use crate::project::ProjectEntry;

/// UI 밀도 모드.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Regular,
    Compact,
}

/// 사용자 테마 선호.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 등록된 프로젝트 레지스트리.
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    /// 활성 프로젝트 id. None 이면 미연결 상태.
    #[serde(default)]
    pub active_project_id: Option<String>,
    pub watch_debounce_ms: u64,
    pub exclude_dirs: Vec<String>,
    pub editor_command: String,
    pub global_shortcut: String,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub sidebar_collapsed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            active_project_id: None,
            watch_debounce_ms: 500,
            exclude_dirs: vec![
                ".git".to_string(),
                ".claude".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
            ],
            editor_command: "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code".to_string(),
            global_shortcut: "CmdOrCtrl+Shift+N".to_string(),
            density: Density::Regular,
            theme: ThemePreference::System,
            sidebar_collapsed: false,
        }
    }
}

pub type ConfigState = Arc<RwLock<AppConfig>>;

pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("config.json")
}

pub fn load_config(app_data_dir: &Path) -> AppConfig {
    let path = config_path(app_data_dir);
    let Ok(content) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    match serde_json::from_str(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("설정 파싱 실패({}): {e}", path.display());
            AppConfig::default()
        }
    }
}

pub fn save_config(config: &AppConfig, app_data_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(app_data_dir)?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;
    fs::write(config_path(app_data_dir), json)?;
    Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AppConfigPatch {
    pub editor_command: Option<String>,
    pub exclude_dirs: Option<Vec<String>>,
    pub global_shortcut: Option<String>,
    pub density: Option<Density>,
    pub theme: Option<ThemePreference>,
    pub sidebar_collapsed: Option<bool>,
}

impl AppConfig {
    pub fn merged_with(&self, patch: AppConfigPatch) -> AppConfig {
        let mut next = self.clone();
        if let Some(v) = patch.editor_command {
            next.editor_command = v;
        }
        if let Some(v) = patch.exclude_dirs {
            next.exclude_dirs = v;
        }
        if let Some(v) = patch.global_shortcut {
            next.global_shortcut = v;
        }
        if let Some(v) = patch.density {
            next.density = v;
        }
        if let Some(v) = patch.theme {
            next.theme = v;
        }
        if let Some(v) = patch.sidebar_collapsed {
            next.sidebar_collapsed = v;
        }
        next
    }

    /// 활성 프로젝트 entry 를 찾아 반환. 없으면 None.
    pub fn active_project(&self) -> Option<&ProjectEntry> {
        let id = self.active_project_id.as_ref()?;
        self.projects.iter().find(|p| &p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_has_no_projects() {
        let config = AppConfig::default();
        assert!(config.projects.is_empty());
        assert!(config.active_project_id.is_none());
    }

    #[test]
    fn default_has_sensible_values() {
        let config = AppConfig::default();
        assert_eq!(config.watch_debounce_ms, 500);
        assert!(config.exclude_dirs.contains(&".git".to_string()));
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let loaded = load_config(dir.path());
        assert!(loaded.projects.is_empty());
    }

    #[test]
    fn load_returns_persisted_config() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            editor_command: "nvim".to_string(),
            ..Default::default()
        };
        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.editor_command, "nvim");
    }

    #[test]
    fn load_returns_default_on_invalid_json() {
        let dir = TempDir::new().unwrap();
        fs::write(config_path(dir.path()), "{not valid json").unwrap();
        let loaded = load_config(dir.path());
        assert!(loaded.projects.is_empty());
    }

    #[test]
    fn load_legacy_v1_config_falls_back_to_empty_projects() {
        // 구 v1 config.json (vault_path / vaults 만 존재) 로딩 시 panic 없이 빈 projects 반환
        let dir = TempDir::new().unwrap();
        let legacy = r#"{
            "vault_path": "/old/vault",
            "vaults": [{"path": "/old/vault", "name": "old"}],
            "watch_debounce_ms": 500,
            "exclude_dirs": [],
            "editor_command": "code",
            "global_shortcut": "CmdOrCtrl+Shift+N"
        }"#;
        fs::write(config_path(dir.path()), legacy).unwrap();
        let loaded = load_config(dir.path());
        assert!(loaded.projects.is_empty());
        assert!(loaded.active_project_id.is_none());
    }

    #[test]
    fn save_creates_config_json_file() {
        let dir = TempDir::new().unwrap();
        save_config(&AppConfig::default(), dir.path()).unwrap();
        assert!(config_path(dir.path()).exists());
    }

    #[test]
    fn save_creates_missing_app_data_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("dir");
        save_config(&AppConfig::default(), &nested).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn merged_with_overrides_some_fields_only() {
        let base = AppConfig::default();
        let patch = AppConfigPatch {
            editor_command: Some("nvim".to_string()),
            ..Default::default()
        };
        let next = base.merged_with(patch);
        assert_eq!(next.editor_command, "nvim");
        assert_eq!(next.global_shortcut, base.global_shortcut);
    }

    #[test]
    fn merged_with_empty_patch_keeps_values() {
        let base = AppConfig {
            editor_command: "code".to_string(),
            ..Default::default()
        };
        let next = base.merged_with(AppConfigPatch::default());
        assert_eq!(next.editor_command, "code");
    }

    #[test]
    fn density_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Density::Compact).unwrap(), "\"compact\"");
    }

    #[test]
    fn theme_pref_default_is_system() {
        assert_eq!(ThemePreference::default(), ThemePreference::System);
    }

    #[test]
    fn merged_with_density_override() {
        let next = AppConfig::default().merged_with(AppConfigPatch {
            density: Some(Density::Compact),
            ..Default::default()
        });
        assert_eq!(next.density, Density::Compact);
    }

    #[test]
    fn merged_with_theme_override() {
        let next = AppConfig::default().merged_with(AppConfigPatch {
            theme: Some(ThemePreference::Dark),
            ..Default::default()
        });
        assert_eq!(next.theme, ThemePreference::Dark);
    }

    #[test]
    fn merged_with_sidebar_override() {
        let next = AppConfig::default().merged_with(AppConfigPatch {
            sidebar_collapsed: Some(true),
            ..Default::default()
        });
        assert!(next.sidebar_collapsed);
    }

    #[test]
    fn active_project_returns_none_when_unset() {
        let config = AppConfig::default();
        assert!(config.active_project_id.is_none());
        assert!(config.active_project().is_none());
    }

    #[test]
    fn active_project_returns_entry_when_set() {
        let entry = crate::project::new_project_entry(PathBuf::from("/tmp/p"), None);
        let id = entry.id.clone();
        let config = AppConfig {
            projects: vec![entry.clone()],
            active_project_id: Some(id),
            ..Default::default()
        };
        assert_eq!(config.active_project(), Some(&entry));
    }

    #[test]
    fn active_project_returns_none_when_id_not_in_registry() {
        let config = AppConfig {
            active_project_id: Some("ghost".to_string()),
            ..Default::default()
        };
        assert!(config.active_project().is_none());
    }
}
