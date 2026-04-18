use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::AppError;

/// 등록된 볼트 항목 (멀티 볼트 관리용)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultEntry {
    pub path: PathBuf,
    pub name: String,
}

/// UI 밀도 모드. 전역 설정으로 앱 전체 간격/폰트를 제어한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Regular,
    Compact,
}

/// 사용자 테마 선호.
/// - `Light`/`Dark`: 명시적 고정
/// - `System`: OS 다크 모드 설정에 연동 (prefers-color-scheme)
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
    /// 현재 활성 볼트 경로. `None`이면 볼트 미연결 상태.
    pub vault_path: Option<PathBuf>,
    /// 등록된 볼트 목록 (멀티 볼트). 구 config에 없으면 빈 Vec.
    #[serde(default)]
    pub vaults: Vec<VaultEntry>,
    pub watch_debounce_ms: u64,
    pub recent_notes_limit: usize,
    pub exclude_dirs: Vec<String>,
    pub editor_command: String,
    pub quick_note_folder: String,
    pub global_shortcut: String,
    /// UI 밀도 (Regular/Compact). 구 config에 없으면 Regular.
    #[serde(default)]
    pub density: Density,
    /// 테마 선호 (Light/Dark/System). 구 config에 없으면 System.
    #[serde(default)]
    pub theme: ThemePreference,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vault_path: None,
            vaults: Vec::new(),
            watch_debounce_ms: 500,
            recent_notes_limit: 20,
            exclude_dirs: vec![
                "dashboard".to_string(),
                ".git".to_string(),
                ".claude".to_string(),
                "_templates".to_string(),
            ],
            editor_command: "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code".to_string(),
            quick_note_folder: "inbox".to_string(),
            global_shortcut: "CmdOrCtrl+Shift+N".to_string(),
            density: Density::Regular,
            theme: ThemePreference::System,
        }
    }
}

/// 런타임에 변경 가능한 설정 상태
pub type ConfigState = Arc<RwLock<AppConfig>>;

/// 설정 파일 경로: `{app_data_dir}/config.json`
pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("config.json")
}

/// 설정을 로드한다. 파일이 없거나 파싱에 실패하면 `Default`를 반환한다.
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

/// 설정을 JSON으로 저장한다. `app_data_dir`이 없으면 자동으로 생성한다.
pub fn save_config(config: &AppConfig, app_data_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(app_data_dir)?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;
    fs::write(config_path(app_data_dir), json)?;
    Ok(())
}

/// 부분 업데이트 patch. `vault_path`는 타입 레벨에서 제외 (온보딩 전용).
/// `watch_debounce_ms`는 v0.5 Settings UI 범위 밖.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AppConfigPatch {
    pub editor_command: Option<String>,
    pub exclude_dirs: Option<Vec<String>>,
    pub recent_notes_limit: Option<usize>,
    pub global_shortcut: Option<String>,
    pub quick_note_folder: Option<String>,
    pub density: Option<Density>,
    pub theme: Option<ThemePreference>,
}

impl AppConfig {
    /// Some인 필드만 현재 설정에 덮어쓴 새 AppConfig 반환.
    pub fn merged_with(&self, patch: AppConfigPatch) -> AppConfig {
        let mut next = self.clone();
        if let Some(v) = patch.editor_command {
            next.editor_command = v;
        }
        if let Some(v) = patch.exclude_dirs {
            next.exclude_dirs = v;
        }
        if let Some(v) = patch.recent_notes_limit {
            next.recent_notes_limit = v;
        }
        if let Some(v) = patch.global_shortcut {
            next.global_shortcut = v;
        }
        if let Some(v) = patch.quick_note_folder {
            next.quick_note_folder = v;
        }
        if let Some(v) = patch.density {
            next.density = v;
        }
        if let Some(v) = patch.theme {
            next.theme = v;
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // BC #1: 기본 설정의 vault_path는 None (볼트 미연결 상태)
    #[test]
    fn default_vault_path_is_none() {
        let config = AppConfig::default();
        assert!(config.vault_path.is_none());
    }

    // BC: Default가 합리적인 값들을 세팅
    #[test]
    fn default_has_sensible_non_vault_values() {
        let config = AppConfig::default();
        assert_eq!(config.watch_debounce_ms, 500);
        assert_eq!(config.recent_notes_limit, 20);
        assert_eq!(config.quick_note_folder, "inbox");
        assert!(config.exclude_dirs.contains(&".git".to_string()));
    }

    // BC #2: 설정 파일 없음 → Default
    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let loaded = load_config(dir.path());
        assert!(loaded.vault_path.is_none());
        assert_eq!(loaded.watch_debounce_ms, 500);
    }

    // BC #3: 유효한 설정 파일 → 파일 내용 반환
    #[test]
    fn load_returns_persisted_config() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            vault_path: Some(PathBuf::from("/tmp/my-vault")),
            recent_notes_limit: 99,
            ..Default::default()
        };

        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());

        assert_eq!(loaded.vault_path, Some(PathBuf::from("/tmp/my-vault")));
        assert_eq!(loaded.recent_notes_limit, 99);
    }

    // BC #4: 잘못된 JSON 파일 → Default
    #[test]
    fn load_returns_default_on_invalid_json() {
        let dir = TempDir::new().unwrap();
        fs::write(config_path(dir.path()), "{not valid json").unwrap();

        let loaded = load_config(dir.path());
        assert!(loaded.vault_path.is_none());
        assert_eq!(loaded.watch_debounce_ms, 500);
    }

    // BC #5: 설정 객체를 저장하면 config.json이 생성됨
    #[test]
    fn save_creates_config_json_file() {
        let dir = TempDir::new().unwrap();
        save_config(&AppConfig::default(), dir.path()).unwrap();
        assert!(config_path(dir.path()).exists());
    }

    // BC #6: save → load roundtrip 시 동일 값 복원
    #[test]
    fn save_then_load_roundtrips() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            vault_path: Some(PathBuf::from("/home/user/vault")),
            editor_command: "nvim".to_string(),
            global_shortcut: "CmdOrCtrl+Alt+Q".to_string(),
            ..Default::default()
        };

        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());

        assert_eq!(loaded.vault_path, cfg.vault_path);
        assert_eq!(loaded.editor_command, cfg.editor_command);
        assert_eq!(loaded.global_shortcut, cfg.global_shortcut);
    }

    // BC #7: app_data_dir 미존재 시 save_config가 디렉토리를 자동 생성
    #[test]
    fn save_creates_missing_app_data_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("dir");
        assert!(!nested.exists());

        save_config(&AppConfig::default(), &nested).unwrap();

        assert!(nested.exists());
        assert!(config_path(&nested).exists());
    }

    // BC #11: Some 필드만 머지, 다른 필드는 유지
    #[test]
    fn merged_with_overrides_some_fields_only() {
        let base = AppConfig::default();
        let patch = AppConfigPatch {
            editor_command: Some("nvim".to_string()),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.editor_command, "nvim");
        assert_eq!(next.quick_note_folder, base.quick_note_folder);
        assert_eq!(next.recent_notes_limit, base.recent_notes_limit);
    }

    // BC #12: 빈 patch → 기존 값 그대로
    #[test]
    fn merged_with_empty_patch_returns_clone() {
        let base = AppConfig {
            editor_command: "code".to_string(),
            recent_notes_limit: 42,
            ..Default::default()
        };
        let next = base.merged_with(AppConfigPatch::default());

        assert_eq!(next.editor_command, "code");
        assert_eq!(next.recent_notes_limit, 42);
    }

    // vault_path는 patch에 없어 영향 없음 (타입 레벨 보호)
    #[test]
    fn merged_with_preserves_vault_path() {
        let base = AppConfig {
            vault_path: Some(PathBuf::from("/v")),
            ..Default::default()
        };
        let patch = AppConfigPatch {
            editor_command: Some("x".to_string()),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.vault_path, Some(PathBuf::from("/v")));
    }

    // watch_debounce_ms도 patch에 없음 → 항상 유지
    #[test]
    fn merged_with_preserves_watch_debounce() {
        let base = AppConfig {
            watch_debounce_ms: 1234,
            ..Default::default()
        };
        let next = base.merged_with(AppConfigPatch::default());
        assert_eq!(next.watch_debounce_ms, 1234);
    }

    // exclude_dirs는 Vec 전체 교체 (patch가 Some이면 완전 덮어쓰기)
    #[test]
    fn merged_with_replaces_exclude_dirs() {
        let base = AppConfig::default();
        let patch = AppConfigPatch {
            exclude_dirs: Some(vec!["only".to_string()]),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.exclude_dirs, vec!["only".to_string()]);
    }

    // ─────────────────────────────────────────
    // Slice 1.2 — Density (컴팩트/기본 밀도)
    // ─────────────────────────────────────────

    // BC #1-2: enum → JSON 소문자
    #[test]
    fn density_serializes_to_lowercase_string() {
        assert_eq!(serde_json::to_string(&Density::Regular).unwrap(), "\"regular\"");
        assert_eq!(serde_json::to_string(&Density::Compact).unwrap(), "\"compact\"");
    }

    // BC #3-4: JSON 소문자 → enum
    #[test]
    fn density_deserializes_from_lowercase_string() {
        assert_eq!(
            serde_json::from_str::<Density>("\"regular\"").unwrap(),
            Density::Regular
        );
        assert_eq!(
            serde_json::from_str::<Density>("\"compact\"").unwrap(),
            Density::Compact
        );
    }

    // BC: default → Regular
    #[test]
    fn density_default_is_regular() {
        assert_eq!(Density::default(), Density::Regular);
    }

    // BC #8: AppConfig::default().density == Regular
    #[test]
    fn app_config_default_density_is_regular() {
        assert_eq!(AppConfig::default().density, Density::Regular);
    }

    // BC #6: 구 config.json (density 필드 없음) → Regular로 로드
    #[test]
    fn load_config_missing_density_field_uses_default() {
        let dir = TempDir::new().unwrap();
        // density 필드 없는 기존 스키마
        let legacy_json = r#"{
            "vault_path": null,
            "vaults": [],
            "watch_debounce_ms": 500,
            "recent_notes_limit": 20,
            "exclude_dirs": [],
            "editor_command": "code",
            "quick_note_folder": "inbox",
            "global_shortcut": "CmdOrCtrl+Shift+N"
        }"#;
        fs::write(config_path(dir.path()), legacy_json).unwrap();

        let loaded = load_config(dir.path());
        assert_eq!(loaded.density, Density::Regular);
    }

    // BC #7: Compact 저장 후 load roundtrip
    #[test]
    fn save_then_load_preserves_compact_density() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            density: Density::Compact,
            ..Default::default()
        };

        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());

        assert_eq!(loaded.density, Density::Compact);
    }

    // BC #9: Some(Compact) patch → Compact 전환
    #[test]
    fn merged_with_applies_some_density() {
        let base = AppConfig::default();
        let patch = AppConfigPatch {
            density: Some(Density::Compact),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.density, Density::Compact);
    }

    // BC #10: None patch → 기존 density 유지
    #[test]
    fn merged_with_none_density_keeps_original() {
        let base = AppConfig {
            density: Density::Compact,
            ..Default::default()
        };
        let next = base.merged_with(AppConfigPatch::default());

        assert_eq!(next.density, Density::Compact);
    }

    // BC #11: Compact → Regular 전환
    #[test]
    fn merged_with_can_switch_compact_to_regular() {
        let base = AppConfig {
            density: Density::Compact,
            ..Default::default()
        };
        let patch = AppConfigPatch {
            density: Some(Density::Regular),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.density, Density::Regular);
    }

    // ─────────────────────────────────────────
    // Slice 1.5 — ThemePreference (라이트/다크/시스템)
    // spec: docs/specs/spec-theme-switcher.md
    // ─────────────────────────────────────────

    // BC #1-3: enum → JSON 소문자
    #[test]
    fn theme_pref_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&ThemePreference::Light).unwrap(),
            "\"light\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreference::Dark).unwrap(),
            "\"dark\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreference::System).unwrap(),
            "\"system\""
        );
    }

    // BC #4: JSON 소문자 → enum
    #[test]
    fn theme_pref_deserializes_from_lowercase() {
        assert_eq!(
            serde_json::from_str::<ThemePreference>("\"light\"").unwrap(),
            ThemePreference::Light
        );
        assert_eq!(
            serde_json::from_str::<ThemePreference>("\"dark\"").unwrap(),
            ThemePreference::Dark
        );
        assert_eq!(
            serde_json::from_str::<ThemePreference>("\"system\"").unwrap(),
            ThemePreference::System
        );
    }

    // BC #5: 잘못된 문자열 거부
    #[test]
    fn theme_pref_rejects_invalid_string() {
        assert!(serde_json::from_str::<ThemePreference>("\"invalid\"").is_err());
        assert!(serde_json::from_str::<ThemePreference>("\"Light\"").is_err()); // 대문자 금지
    }

    // BC #6: default → System
    #[test]
    fn theme_pref_default_is_system() {
        assert_eq!(ThemePreference::default(), ThemePreference::System);
    }

    // BC #9: AppConfig::default().theme == System
    #[test]
    fn app_config_default_theme_is_system() {
        assert_eq!(AppConfig::default().theme, ThemePreference::System);
    }

    // BC #7: 구 config.json (theme 필드 없음) → System
    #[test]
    fn load_config_missing_theme_field_uses_default() {
        let dir = TempDir::new().unwrap();
        let legacy_json = r#"{
            "vault_path": null,
            "vaults": [],
            "watch_debounce_ms": 500,
            "recent_notes_limit": 20,
            "exclude_dirs": [],
            "editor_command": "code",
            "quick_note_folder": "inbox",
            "global_shortcut": "CmdOrCtrl+Shift+N",
            "density": "regular"
        }"#;
        fs::write(config_path(dir.path()), legacy_json).unwrap();

        let loaded = load_config(dir.path());
        assert_eq!(loaded.theme, ThemePreference::System);
    }

    // BC #8: Light 저장 후 load roundtrip
    #[test]
    fn save_then_load_preserves_light_theme() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            theme: ThemePreference::Light,
            ..Default::default()
        };

        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());

        assert_eq!(loaded.theme, ThemePreference::Light);
    }

    // BC #8: Dark 저장 후 load roundtrip
    #[test]
    fn save_then_load_preserves_dark_theme() {
        let dir = TempDir::new().unwrap();
        let cfg = AppConfig {
            theme: ThemePreference::Dark,
            ..Default::default()
        };

        save_config(&cfg, dir.path()).unwrap();
        let loaded = load_config(dir.path());

        assert_eq!(loaded.theme, ThemePreference::Dark);
    }

    // BC #10: Some(theme) patch → 반영
    #[test]
    fn merged_with_applies_some_theme() {
        let base = AppConfig::default(); // System
        let patch = AppConfigPatch {
            theme: Some(ThemePreference::Light),
            ..Default::default()
        };
        let next = base.merged_with(patch);

        assert_eq!(next.theme, ThemePreference::Light);
    }

    // BC #11: None patch → 기존 theme 유지
    #[test]
    fn merged_with_none_theme_keeps_original() {
        let base = AppConfig {
            theme: ThemePreference::Dark,
            ..Default::default()
        };
        let next = base.merged_with(AppConfigPatch::default());

        assert_eq!(next.theme, ThemePreference::Dark);
    }

    // BC #12: 전환 가능 — Light → System, Dark → Light 등
    #[test]
    fn merged_with_can_switch_any_theme() {
        // Light → System
        let light_base = AppConfig {
            theme: ThemePreference::Light,
            ..Default::default()
        };
        let to_system = light_base.merged_with(AppConfigPatch {
            theme: Some(ThemePreference::System),
            ..Default::default()
        });
        assert_eq!(to_system.theme, ThemePreference::System);

        // Dark → Light
        let dark_base = AppConfig {
            theme: ThemePreference::Dark,
            ..Default::default()
        };
        let to_light = dark_base.merged_with(AppConfigPatch {
            theme: Some(ThemePreference::Light),
            ..Default::default()
        });
        assert_eq!(to_light.theme, ThemePreference::Light);
    }
}
