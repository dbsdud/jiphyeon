# Spec: vaults (v0.5 멀티 볼트 관리)

## 전제

- 앱은 **등록된 볼트 목록**을 관리한다. 한 번에 하나가 **활성**(= `vault_path`).
- `create_vault` / `connect_vault` 성공 시 자동으로 목록에 추가 (중복 제거).
- 사용자는 사이드바에서 클릭으로 전환, × 버튼으로 제거한다.
- **활성 볼트는 제거할 수 없다** — 먼저 다른 볼트로 전환해야 한다.
- v0.5에선 `VaultEntry.name`은 **폴더명에서 자동 추출** (편집 UI는 v0.6).
- 경로 정규화: **trailing slash 제거 + 절대경로**. `canonicalize`는 하지 않음 (심볼릭 링크 해석 부작용 회피).

## Public Interface

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultEntry {
    pub path: PathBuf,
    pub name: String,   // 폴더명 자동 추출 (예: "/foo/my-vault" → "my-vault")
}

// AppConfig에 필드 추가 (serde_default로 migration-safe)
#[serde(default)]
pub vaults: Vec<VaultEntry>,

// IPC 커맨드 (commands/vaults.rs)

#[tauri::command]
pub fn list_vaults(config_state: State<'_, ConfigState>) -> Result<Vec<VaultEntry>, AppError>

#[tauri::command]
pub fn switch_vault(
    config_state: State<'_, ConfigState>,
    vault_state: State<'_, VaultState>,
    search_state: State<'_, SearchState>,
    watcher_state: State<'_, WatcherState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError>

#[tauri::command]
pub fn remove_vault(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<VaultEntry>, AppError>

// --- 순수 헬퍼 (유닛 테스트 대상) ---

/// 경로를 정규화한다 (trailing slash 제거, to 절대경로 변환).
pub fn normalize_vault_path(input: &Path) -> PathBuf

/// 폴더명에서 표시용 이름 추출 (끝 세그먼트). 빈 경우 "vault" fallback.
pub fn derive_vault_name(path: &Path) -> String

/// vaults에 중복 없이 push (첫 등장 유지). 정규화된 path 기준 비교.
pub fn upsert_vault(vaults: &mut Vec<VaultEntry>, entry: VaultEntry)
```

## Invariants

- `AppConfig::default().vaults`는 빈 Vec
- 경로 비교는 항상 정규화된 `PathBuf` 기준 (중복 판정)
- `create_vault`/`connect_vault`가 성공했다면 **반드시** `vaults`에 해당 경로가 있다
- `switch_vault`가 성공했다면 `vault_path == Some(target)` + 해당 경로가 `vaults`에 있다
- `remove_vault`는 `vaults`에서만 제거하고 **실제 파일시스템은 건드리지 않는다**
- 활성 볼트(= `vault_path`)를 `remove_vault` 하면 `VaultNotFound` 에러 (가까운 의미 재사용) + 목록 변경 없음

## Behavior Contract — normalize_vault_path

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `/foo/bar/` (trailing slash) | `normalize_vault_path` | `/foo/bar` |
| 2 | `/foo/bar` | `normalize_vault_path` | `/foo/bar` (동일) |
| 3 | 상대경로 `./vault` | `normalize_vault_path` | 현재 디렉토리 기준 절대경로로 변환 |
| 4 | 이미 절대경로 | `normalize_vault_path` | 그대로 |

## Behavior Contract — derive_vault_name

| # | Given | When | Then |
|---|-------|------|------|
| 5 | `/foo/my-vault` | `derive_vault_name` | `"my-vault"` |
| 6 | `/foo/my-vault/` | `derive_vault_name` | `"my-vault"` (trailing slash 무시) |
| 7 | `/` (루트) | `derive_vault_name` | `"vault"` (fallback) |

## Behavior Contract — upsert_vault

| # | Given | When | Then |
|---|-------|------|------|
| 8 | 빈 vaults, entry A | `upsert_vault` | `[A]` |
| 9 | `[A]`, 동일 경로 entry A' (name 달라도) | `upsert_vault` | `[A]` (첫 등장 유지, 변경 없음) |
| 10 | `[A]`, 다른 경로 entry B | `upsert_vault` | `[A, B]` |

## Behavior Contract — create_vault / connect_vault 확장

| # | Given | When | Then |
|---|-------|------|------|
| 11 | `vaults: []`, 새 경로 create 성공 | `create_vault` | `vaults.len() == 1`, 새 경로 포함 |
| 12 | `vaults: [A]`, 이미 등록된 A를 connect 재호출 | `connect_vault` | `vaults.len() == 1` (중복 제거) |
| 13 | 서로 다른 경로로 순차 create → connect | — | `vaults.len() == 2` |

## Behavior Contract — list_vaults

| # | Given | When | Then |
|---|-------|------|------|
| 14 | 볼트 미연결 (`vault_path: None`, `vaults: []`) | `list_vaults` | 빈 Vec |
| 15 | 등록된 3개 | `list_vaults` | 3개 반환, 순서는 등록 순 유지 |

## Behavior Contract — switch_vault

| # | Given | When | Then |
|---|-------|------|------|
| 16 | 목록에 있는 경로로 전환 | `switch_vault` | `vault_path`가 해당 경로로 변경, 인덱스/검색/watcher 재구축, VaultStatus 반환 |
| 17 | 목록에 없는 경로로 전환 시도 | `switch_vault` | `AppError::VaultNotFound` (목록에서 관리된 볼트만 전환 허용) |
| 18 | 이미 활성인 경로로 전환 | `switch_vault` | 정상 (no-op이지만 reactivate로 인덱스 rebuild — 사용자 명시 액션 존중) |

## Behavior Contract — remove_vault

| # | Given | When | Then |
|---|-------|------|------|
| 19 | 비활성 볼트 제거 | `remove_vault` | `vaults`에서 제거, 반환은 갱신된 목록, 파일시스템은 무변경 |
| 20 | 활성 볼트 제거 시도 | `remove_vault` | `AppError::VaultNotFound` (의미상 "제거할 수 없음"), 목록 무변경 |
| 21 | 목록에 없는 경로 제거 시도 | `remove_vault` | 현재 목록 그대로 반환, 에러 없음 (idempotent) |
| 22 | 제거 후 | `load_config` | 변경이 영속화된 목록 반환 |

## Edge Cases

- 활성 볼트 `vault_path`가 `vaults`에 없는 경우(구버전 config) — `list_vaults` 호출 시 자동 upsert로 채움 (migration-safe)
- `create_vault`/`connect_vault` 실패 → `vaults` 변경 없음 (원자성)
- `switch_vault` 중 인덱싱 실패 → 설정의 `vault_path`를 기존 값으로 롤백? **v0.5 비범위** (간단히 에러 전파 + 상태 일관성은 프론트가 reload로 회복)
- 볼트 디렉토리가 삭제된 상태에서 `switch_vault` 호출 → `scan_vault`가 `VaultNotFound` 반환. 목록에는 남음 (v0.5는 수동 제거 필요)

## Dependencies

- 기존 `config::{save_config, ConfigState, AppConfig}`
- 기존 `commands::onboarding::activate_vault` 재사용 (`switch_vault` 구현의 핵심)
- Mock boundary: 파일 시스템 (TempDir 통합 테스트)

## 기존 코드 영향

- `config.rs`
  - `AppConfig`에 `vaults: Vec<VaultEntry>` 필드 추가 (`#[serde(default)]`)
  - `AppConfig::default().vaults`는 `vec![]`
- `commands/onboarding.rs`
  - `activate_vault`에서 성공 시 `upsert_vault` 호출 → 설정에 등록
  - `pub(crate)` 수준으로 노출 (다른 커맨드에서 재사용)
- `commands/mod.rs`, `lib.rs`
  - 새 모듈 `commands::vaults` 등록
  - 3개 커맨드 handler 추가

## Mock boundary

- `normalize_vault_path`, `derive_vault_name`, `upsert_vault`: 순수 함수 → 문자열/Vec 조작 유닛 테스트
- `list_vaults`/`switch_vault`/`remove_vault`: ConfigState + 파일 I/O → TempDir 통합 테스트는 어려움. 핵심 머지 로직은 헬퍼에 격리해 단위화.

## UI 노출 (D2)

### 사이드바
- 상단 섹션 "📓 볼트" + "+" 버튼
- 목록 렌더링:
  - 활성 볼트: ● 표시 + 굵게
  - 비활성: ○ + hover 시 × 제거 버튼
- "+" 클릭 → 새 볼트 생성/기존 연결 선택 (모달) — 기존 온보딩 로직 재사용
- 클릭 전환 후 `window.location.reload()`

### 대시보드
- 상단에 활성 볼트명 표시 (사용자 요청)
- 예: `Dashboard — my-vault-test`

### Settings
- 기존 볼트 섹션은 **현재 볼트 표시 + 버튼**만 남기고, 목록 관리는 사이드바에서 (중복 제거)
