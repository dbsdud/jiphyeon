# Spec: config (v0.4 설정 영속화)

## Public Interface

```rust
/// 앱 설정. vault_path가 None이면 볼트 미연결 상태.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub vault_path: Option<PathBuf>,  // 변경: PathBuf → Option<PathBuf>
    pub watch_debounce_ms: u64,
    pub recent_notes_limit: usize,
    pub exclude_dirs: Vec<String>,
    pub editor_command: String,
    pub quick_note_folder: String,
    pub global_shortcut: String,
}

/// 런타임 변경 가능한 설정 상태
pub type ConfigState = Arc<RwLock<AppConfig>>;

/// 설정 파일 경로 반환: {app_data_dir}/config.json
pub fn config_path(app_data_dir: &Path) -> PathBuf

/// 설정 로드. 파일 없거나 파싱 실패 시 Default 반환.
pub fn load_config(app_data_dir: &Path) -> AppConfig

/// 설정을 JSON으로 저장.
pub fn save_config(config: &AppConfig, app_data_dir: &Path) -> Result<(), AppError>
```

## Invariants

- `AppConfig::default().vault_path`는 `None`
- `save_config` → `load_config` roundtrip 시 동일 값 복원
- 설정 파일이 없거나 잘못된 JSON이면 Default 반환 (panic 없음)

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 기본 설정 | `Default::default()` | `vault_path == None` |
| 2 | 설정 파일 없음 | `load_config` | Default 반환 |
| 3 | 유효한 설정 파일 | `load_config` | 파일 내용 반환 |
| 4 | 잘못된 JSON 파일 | `load_config` | Default 반환 |
| 5 | 설정 객체 | `save_config` | `{app_data_dir}/config.json` 생성 |
| 6 | save 후 | `load_config` | 동일 설정 반환 (roundtrip) |
| 7 | app_data_dir 미존재 | `save_config` | 디렉토리 자동 생성 후 저장 |

## Edge Cases

- `app_data_dir`가 존재하지 않으면 `save_config`에서 `create_dir_all`
- JSON 파싱 실패 시 에러 로그 출력 후 Default 반환 (사용자 경험 보호)

## Dependencies

- `serde_json`: JSON 직렬화/역직렬화
- `std::fs`: 파일 I/O
- Mock boundary: 없음 (순수 파일 I/O)

## 기존 코드 영향

`config.vault_path`를 직접 참조하는 모든 코드에서 `Option` 처리 필요:
- `commands/vault.rs`: `get_recent_notes`, `rescan_vault`
- `commands/note.rs`: `get_note`, `open_in_editor`, `create_quick_note`
- `lib.rs`: `scan_vault`, `start_watching` 호출부
- `watcher/mod.rs`: `config.vault_path` 접근

에러 처리 패턴: `config.vault_path.as_ref().ok_or(AppError::VaultNotConfigured)?`
