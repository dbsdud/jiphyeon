# Spec: onboarding (v0.4 볼트 생성/연결)

## Public Interface

```rust
/// 볼트 연결 상태
#[derive(Serialize)]
pub struct VaultStatus {
    pub connected: bool,
    pub vault_path: Option<String>,
}

/// 볼트 디렉토리 구조 생성 (순수 함수, 테스트 가능)
pub fn scaffold_vault(root: &Path) -> Result<(), AppError>

/// IPC: 볼트 연결 상태 조회
#[tauri::command]
pub fn get_vault_status(config: State<ConfigState>) -> Result<VaultStatus, AppError>

/// IPC: 새 볼트 생성 (scaffold + 활성화)
#[tauri::command]
pub fn create_vault(
    config_state: State<ConfigState>,
    vault_state: State<VaultState>,
    search_state: State<SearchState>,
    watcher_state: State<WatcherState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError>

/// IPC: 기존 볼트 연결 (활성화만)
#[tauri::command]
pub fn connect_vault(
    config_state: State<ConfigState>,
    vault_state: State<VaultState>,
    search_state: State<SearchState>,
    watcher_state: State<WatcherState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError>
```

## 내부 함수

```rust
/// create_vault/connect_vault 공통 후처리
fn activate_vault(
    config_state: &ConfigState,
    vault_state: &VaultState,
    search_state: &SearchState,
    watcher_state: &WatcherState,
    app_handle: &AppHandle,
    vault_path: PathBuf,
    app_data_dir: &Path,
) -> Result<VaultStatus, AppError>
```

## Invariants

- `scaffold_vault`는 기존 파일을 덮어쓰지 않음
- `activate_vault` 후 설정이 영속화됨 (앱 재시작 시 유지)
- `activate_vault` 후 vault_state, search_state, watcher가 갱신됨

## Behavior Contract — scaffold_vault

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 빈 디렉토리 | `scaffold_vault` | 11개 디렉토리 생성 |
| 2 | 빈 디렉토리 | `scaffold_vault` | .gitignore, .gitattributes, CLAUDE.md 생성 |
| 3 | 빈 디렉토리 | `scaffold_vault` | _moc/ 에 4개 MOC 파일 생성 |
| 4 | 빈 디렉토리 | `scaffold_vault` | _templates/ 에 9개 템플릿 파일 생성 |
| 5 | 이미 파일 있는 디렉토리 | `scaffold_vault` | 기존 파일 보존, 없는 것만 생성 |
| 6 | 존재하지 않는 경로 | `scaffold_vault` | `AppError::VaultNotFound` 반환 |

## Behavior Contract — IPC 커맨드

| # | Given | When | Then |
|---|-------|------|------|
| 7 | vault_path == None | `get_vault_status` | `{ connected: false, vault_path: null }` |
| 8 | vault_path == Some | `get_vault_status` | `{ connected: true, vault_path: "..." }` |
| 9 | 유효한 경로 | `create_vault` | scaffold + activate, `connected: true` 반환 |
| 10 | 유효한 볼트 경로 | `connect_vault` | activate, `connected: true` 반환 |
| 11 | 존재하지 않는 경로 | `connect_vault` | `AppError::VaultNotFound` 반환 |

## scaffold_vault 생성물

### 디렉토리 (11개)
inbox, dev, decisions, readings, meetings, ideas, artifacts, projects, _moc, _templates, _maintenance

### 루트 파일 (3개)
- `.gitignore` — OS/Tauri/Claude 무시 패턴
- `.gitattributes` — `_maintenance/reports/** merge=ours`
- `CLAUDE.md` — 볼트 구조 및 규칙 문서

### MOC 파일 (4개)
- `_moc/Home.md`, `_moc/Topics.md`, `_moc/Projects.md`, `_moc/Timeline.md`

### 템플릿 파일 (9개)
- `_templates/tpl-artifact.md`, `tpl-clipping.md`, `tpl-decision.md`, `tpl-idea.md`, `tpl-meeting.md`, `tpl-project-moc.md`, `tpl-reading.md`, `tpl-til.md`, `tpl-topic-moc.md`

## Edge Cases

- 볼트 경로에 한글/공백이 포함된 경우 → PathBuf로 처리되므로 문제 없음
- scaffold 중 일부 파일 생성 실패 → 에러 전파 (partial scaffold 가능성 있음)
- activate 중 scan_vault 실패 → 에러 반환 (설정은 이미 저장된 상태)

## Dependencies

- `config.rs`: `ConfigState`, `save_config`, `load_config`
- `vault/indexer.rs`: `scan_vault`
- `vault/search.rs`: `build_search_index`
- `watcher/mod.rs`: `start_watching`
- Mock boundary: 없음 (파일 시스템 직접 접근)
