# Spec: Project Registry (Slice B-1)

**상태**: Draft
**작성일**: 2026-04-25
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Epic B / Slice B-1

## 목표

v1.0의 단일 Vault 모델을 v2.0의 멀티 Project 모델로 교체한다.
사용자는 임의 폴더(주로 GitHub 레포)를 "프로젝트"로 등록하고 활성 프로젝트를 전환한다.
집현은 활성 프로젝트의 `docs/` 폴더(노트)와 `graphify-out/` 폴더(graphify 산출물)를 데이터 소스로 본다.

## 배경

- v1.0은 `AppConfig.vault_path: Option<PathBuf>` + `vaults: Vec<VaultEntry>` 구조였다.
- VaultEntry는 `path` + `name`만 가졌고, 단일 폴더 안에 모든 노트가 있다고 가정했다.
- v2.0은 (a) docs/ 와 graphify-out/ 을 분리해 인식하고 (b) 프로젝트별 graphify 실행 시각을 추적해야 한다.
- v1.0 사용자는 없으므로 마이그레이션 코드는 작성하지 않는다. 기존 `config.json`은 `Default` 로 폴백된다.

## 데이터 모델

```rust
// src-tauri/src/models.rs (또는 config.rs)
pub struct ProjectEntry {
    pub id: String,                  // path 해시 기반(deterministic) — 동일 경로 재등록 시 동일 id
    pub name: String,                // 사용자 지정 또는 폴더명
    pub root_path: PathBuf,          // 프로젝트 루트 (정규화된 절대 경로)
    pub docs_path: PathBuf,          // 기본값: root_path/docs
    pub graphify_out_path: PathBuf,  // 기본값: root_path/graphify-out
    pub registered_at: String,       // ISO 8601 (RFC 3339, UTC)
    pub last_graphify_at: Option<String>, // graphify-out/graph.json 의 mtime (없으면 None)
}

pub struct AppConfig {
    pub projects: Vec<ProjectEntry>,
    pub active_project_id: Option<String>,
    // 기존 UI 프리퍼런스는 유지
    pub theme: ThemePreference,
    pub density: Density,
    pub sidebar_collapsed: bool,
    pub editor_command: String,
    pub watch_debounce_ms: u64,
    pub global_shortcut: String,
    pub exclude_dirs: Vec<String>,
    // 제거: vault_path, vaults, recent_notes_limit, quick_note_folder
}
```

### 필드 결정 사항

- **id**: `blake3(root_path 정규화)` 의 hex 16자. 별도 UUID 의존성 추가 회피, deterministic 보장.
- **name**: 등록 시 사용자가 지정하면 그 값, 없으면 `root_path.file_name()`.
- **registered_at / last_graphify_at**: chrono `DateTime<Utc>` 를 RFC3339 String으로 직렬화. 별도 `chrono` serde 피처 회피.
- **graphify_out_path** 기본값: 로드맵의 "graphify-out 기본 경로: `<project>/graphify-out/` (cwd 기준으로 생성됨)" 결정에 따라 `root_path/graphify-out`.
- **recent_notes_limit / quick_note_folder**: v2.0에서 더 이상 사용 안 함 → 제거. quick note 저장 폴더는 Epic D에서 활성 프로젝트의 `docs/inbox/`로 고정.

## Public Interface

### Pure Helpers (`src-tauri/src/project.rs` 신규 모듈)

```rust
/// path 정규화 (vaults.rs::normalize_vault_path 와 동일 로직 재사용)
pub fn normalize_root(path: &Path) -> PathBuf;

/// 결정적 id 생성. 동일 정규화 경로 → 동일 id.
pub fn derive_project_id(root_path: &Path) -> String;

/// 폴더명 → 표시 이름. 추출 실패 시 "project" fallback.
pub fn derive_project_name(root_path: &Path) -> String;

/// graphify-out/graph.json 파일의 mtime 을 RFC3339로 반환. 파일 없으면 None.
pub fn read_last_graphify_at(graphify_out_path: &Path) -> Option<String>;

/// 새 ProjectEntry 생성 (id + 시각 자동 채움).
pub fn new_project_entry(root_path: PathBuf, name: Option<String>) -> ProjectEntry;
```

### Tauri 커맨드 (`src-tauri/src/commands/projects.rs` 신규)

```rust
#[tauri::command]
pub fn list_projects(...) -> Result<Vec<ProjectEntry>, AppError>;

#[tauri::command]
pub fn register_project(
    root_path: String,
    name: Option<String>,
    create_docs: bool,
) -> Result<ProjectEntry, AppError>;

#[tauri::command]
pub fn switch_project(id: String) -> Result<ProjectEntry, AppError>;

#[tauri::command]
pub fn remove_project(id: String) -> Result<Vec<ProjectEntry>, AppError>;

#[tauri::command]
pub fn get_active_project(...) -> Result<Option<ProjectEntry>, AppError>;
```

## Behavior Contract

### `derive_project_id`
- Given: 정규화된 절대 경로
- When: 동일 경로 두 번 호출
- Then: 동일 hex 문자열 (16자)
- Given: 두 다른 경로
- When: 호출
- Then: 서로 다른 문자열

### `derive_project_name`
- Given: `/Users/uno/work/my-project`
- When: 호출
- Then: `"my-project"`
- Given: `/`
- When: 호출
- Then: `"project"` (fallback)

### `read_last_graphify_at`
- Given: `<dir>/graph.json` 이 존재
- When: 호출
- Then: `Some(rfc3339 mtime)`
- Given: `<dir>/graph.json` 미존재
- When: 호출
- Then: `None`
- Given: `<dir>` 자체가 없음
- When: 호출
- Then: `None` (에러 X)

### `register_project`
- Given: 존재하는 폴더, `create_docs=true`, `docs/` 미존재
- When: 호출
- Then: `docs/` 생성됨, ProjectEntry 반환, AppConfig.projects 에 추가됨, active_project_id 가 새 id 로 설정, config.json 디스크 저장됨
- Given: 존재하는 폴더, `create_docs=false`, `docs/` 미존재
- When: 호출
- Then: 에러 `AppError::DocsNotFound` (또는 등가)
- Given: 동일 root_path 가 이미 등록됨
- When: 재등록
- Then: 기존 항목 유지(중복 추가 X), 활성 프로젝트만 그 id 로 전환
- Given: 존재하지 않는 폴더
- When: 호출
- Then: 에러 `AppError::VaultNotFound` 재사용 또는 신규 `AppError::ProjectRootNotFound`

### `switch_project`
- Given: 등록된 id
- When: 호출
- Then: `active_project_id` 갱신됨, 변경 후 ProjectEntry 반환, config 저장
- Given: 미등록 id
- When: 호출
- Then: 에러

### `remove_project`
- Given: 활성 아닌 등록 id
- When: 호출
- Then: 레지스트리에서만 제거, 파일 시스템 변경 없음, 갱신된 목록 반환
- Given: 활성 id
- When: 호출
- Then: 에러 (먼저 다른 프로젝트로 전환 요구) — Epic A 의 remove_vault 패턴 재사용

### `list_projects`
- Given: 빈 레지스트리
- When: 호출
- Then: `Vec::new()`
- Given: N개 등록
- When: 호출
- Then: 등록 순서대로 N개 반환. 호출 시점에 각 항목의 `last_graphify_at` 을 다시 계산하여 최신 값 포함.

### `get_active_project`
- Given: active_project_id 가 None
- When: 호출
- Then: `Ok(None)`
- Given: active_project_id 가 등록된 id
- When: 호출
- Then: `Ok(Some(entry))` (last_graphify_at 재계산 포함)
- Given: active_project_id 가 등록 안 된 id (corrupt config)
- When: 호출
- Then: `Ok(None)` (graceful)

## Edge Cases

- **동시 호출**: register_project 동시 두 번 → `Vec` 순차 잠금 후 동일 root는 upsert 로직으로 idempotent
- **id 충돌**: 별개 path 에서 16자 hex 충돌은 통계적 무시 가능. 충돌 시 register는 실패 응답(같은 id 가 이미 다른 path 에 매핑됨) — 우선 detect 후 패닉 X, 에러 반환
- **root_path 가 심볼릭 링크**: normalize_root 가 `canonicalize` 까지 하지 않음 (vault 시절과 동일 정책). 사용자가 같은 폴더를 다른 심볼릭 경로로 등록하면 별개 프로젝트로 인식됨 — 의도된 동작
- **docs/ 가 파일(폴더 아님)**: register 시 `AppError::Io` 로 폴백
- **graph.json mtime 읽기 실패**: `read_last_graphify_at` 이 `None` 반환, 절대 panic 안 함

## Dependencies

- `blake3` 크레이트 추가 (Cargo.toml `src-tauri`)
- `chrono` 는 이미 의존, `serde` feature 만 활용
- 기존 `vaults.rs::normalize_vault_path` 로직은 `project::normalize_root` 로 이전 후 vaults.rs는 Epic B-1 완료 시 삭제
- Mock boundary 없음 (file system 직접 조작; 테스트는 `tempfile::TempDir` 사용)

## 마이그레이션 / 호환성

- 기존 `config.json` 의 `vault_path` / `vaults` 필드는 새 스키마에서 무시됨.
- `serde(default)` 로 누락 필드는 Default 사용 → 구 config 가 있어도 panic 없이 빈 projects 로 시작.
- v1 사용자가 없으므로 자동 마이그레이션 함수는 작성 안 함. 단, 자체 테스트로 "구 vault_path 만 있는 JSON" 로드 시 빈 projects 로 폴백되는 것만 검증.

## 비범위 (이 Slice 에서 다루지 않는 것)

- `ProjectOnboarding.svelte` UI 작성 → Slice B-2
- 사이드바 프로젝트 스위처 / `/explore` 폴더 트리 재배선 → Slice B-3
- graphify-out 의 graph.json 파싱 → Epic C-1
- watcher 대상 변경 (vault_path → 활성 프로젝트의 graphify-out 등) → Slice B-3 또는 Epic C-1
- 기존 `commands/onboarding.rs`, `commands/vaults.rs`, `clip_url`, `quick_note` 의 vault_path 의존 정리 → Slice B-1 마무리에서 한 번에 처리 (config 스키마 교체 시 컴파일 에러 발생 → 직진 수정)

## 작업 순서 (TDD)

1. `project::normalize_root`, `derive_project_id`, `derive_project_name`, `new_project_entry` 의 단위 테스트 → 구현
2. `read_last_graphify_at` 단위 테스트 (TempDir + 실 파일) → 구현
3. AppConfig 스키마 교체. 구 vault 관련 필드 제거. config.rs 테스트 갱신
4. AppConfig load/save roundtrip + 빈 config 폴백 테스트
5. `commands/projects.rs` 에 `register_project` / `switch_project` / `remove_project` / `list_projects` / `get_active_project` 단위 + 통합 테스트
6. `lib.rs::setup` 의 watcher 시작 분기 수정 (vault_path → active project root). 활성 프로젝트 없으면 watcher 미기동
7. 컴파일 에러 추적 — 기존 `commands/onboarding.rs`, `commands/vaults.rs`, `clipper.rs`, `commands/note.rs`(create_quick_note), `lib.rs` 의 vault_path 의존 코드 모두 active project 기반으로 교체 또는 stub
8. `cargo test --lib` 통과 → svelte-check 통과 (프론트엔드는 Slice B-2/B-3 까지 임시 build 깨질 수 있으므로 별도 점검)
9. 커밋: `refactor: AppConfig 를 Project 레지스트리로 교체 (Slice B-1)`

## 완료 조건

- `cargo test --lib` 전체 통과
- `commands/projects.rs` 모든 BC 테스트 green
- 구 `vault_path` / `VaultEntry` / `commands/vaults.rs` / `commands/onboarding.rs::connect_vault` 등 잔재 제거
- (프론트는 임시 깨짐 허용 — Slice B-2/B-3에서 복구)
