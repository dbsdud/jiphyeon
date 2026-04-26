# Spec: Workspace Hub (v2.5)

**상태**: Draft
**작성일**: 2026-04-26
**브랜치**: `feat/v2.5-workspace-hub`
**선행**: Epic A, B 완료
**연관 결정 변경**: v2.0 로드맵 "확정 설계 결정" 보강 — Hub 디렉토리 도입

## 목표

흩어진 프로젝트(레포)를 사용자의 홈 아래 단일 디렉토리(`~/Jiphyeon/`)에 symlink 로 모아, 집현·터미널·Finder 어디에서든 하나의 hub 로 보이게 한다. 등록한 프로젝트는 자동으로 `~/Jiphyeon/<name>` 으로 노출되고, 모든 fs 접근(노트 조회·watcher·graphify-out 읽기)은 hub 경유로 이루어진다.

## 사용자 시나리오

1. 집현을 처음 실행 → `~/Jiphyeon/` 이 없으면 자동 생성 후 안내
2. 사이드바에서 "프로젝트 추가" → 폴더 다이얼로그로 임의 위치(예: `~/work/foo`) 선택
3. 집현이 `~/Jiphyeon/foo` symlink 를 만들고 등록
4. 사용자는 `cd ~/Jiphyeon/foo` 에서 Claude Code 를 열고 `/graphify` 실행 → graphify-out 이 실제 `~/work/foo/graphify-out/` 에 생성되지만 `~/Jiphyeon/foo/graphify-out/` 으로도 접근 가능
5. 집현 사이드바·`/explore` 는 모두 hub 경로 기준으로 동작
6. Finder 에서 `~/Jiphyeon/` 을 열면 모든 등록 프로젝트가 한 폴더에 정렬

## 결정 사항 (사용자 확정)

| # | 항목 | 값 |
|---|---|---|
| A | workspace 위치 | 고정 `~/Jiphyeon` (변경 불가) |
| B | 등록 시 symlink | 자동 생성 (옵션 X) |
| C | 윈도우 지원 | macOS/Linux 우선, Windows 후순위 |
| D | graphify 실행 주체 | 집현 X — 사용자가 hub 경로에서 Claude Code 실행 |
| E | 기존 등록 마이그레이션 | 다음 실행 시 자동 best-effort symlink |

## 데이터 모델 변경

```rust
// src-tauri/src/config.rs
pub struct AppConfig {
    pub workspace_path: PathBuf,   // 신규. 기본: dirs::home_dir().join("Jiphyeon")
    pub projects: Vec<ProjectEntry>,
    pub active_project_id: Option<String>,
    // 기존 필드 유지
}
```

```rust
// src-tauri/src/project.rs
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,        // 실제 폴더 (symlink target)
    pub link_path: PathBuf,        // 신규. ~/Jiphyeon/<name>
    pub docs_path: PathBuf,        // = link_path/docs
    pub graphify_out_path: PathBuf,// = link_path/graphify-out
    pub registered_at: String,
    pub last_graphify_at: Option<String>,
}
```

**중요한 변화**: `docs_path` / `graphify_out_path` 가 `link_path` 기준으로 변경됨. fs 호출은 OS 가 symlink 를 자동 follow 하므로 본문은 실제 `root_path/docs/...` 의 내용을 그대로 읽는다.

## Public Interface

### 신규 헬퍼 (`src-tauri/src/workspace.rs` 신규)

```rust
/// 기본 workspace 경로: $HOME/Jiphyeon. HOME 미설정 시 폴백.
pub fn default_workspace_path() -> PathBuf;

/// 디렉토리가 없으면 생성 (mkdir -p). 존재하지만 폴더가 아니면 에러.
pub fn ensure_workspace_dir(path: &Path) -> Result<(), AppError>;

/// `<workspace>/<requested_name>` 에 `target` 으로의 symlink 를 생성한다.
/// - 동일 target 의 symlink 가 이미 있으면 idempotent (재사용)
/// - 다른 target 이거나 일반 폴더가 점유 → 자동 suffix `<name>-2`, `<name>-3`...
/// - 반환: 실제 사용된 link path
pub fn create_project_symlink(
    workspace: &Path,
    requested_name: &str,
    target: &Path,
) -> Result<PathBuf, AppError>;

/// link 가 깨졌는지 (target 이 사라졌는지) 확인.
pub fn is_link_broken(link_path: &Path) -> bool;
```

### `register_project` 시그니처/동작 변경

- 입력은 동일 (`root_path: String`, `name: Option<String>`, `create_docs: bool`)
- 동작:
  1. target 폴더 정규화 + docs/ 처리 (기존)
  2. **`create_project_symlink(workspace, derive_name(target), target)` 호출** → `link_path` 확보
  3. `ProjectEntry` 의 `docs_path` / `graphify_out_path` 는 `link_path` 기준으로 채움
  4. 기존 idempotent 로직: derive_project_id 는 target 의 실제 경로 기준 (현재와 동일) → 같은 root 재등록은 그대로 idempotent

### 첫 실행 / 마이그레이션

- `lib.rs::setup` 에서:
  1. `workspace_path` 가 `AppConfig` 에 없으면 default 채움
  2. `ensure_workspace_dir(workspace_path)`
  3. 등록된 모든 프로젝트에 대해 `link_path` 가 존재 안 하거나 비어 있으면 best-effort symlink 재생성. 실패 시 로그만, 부팅 진행.

### Watcher

- 기존: `start_watching(active_project.root_path, ...)`
- 신규: `start_watching(workspace_path, ...)` — workspace 전체 감시. notify 가 symlink 를 따라가도록 옵션 ON.
- 이벤트의 path 는 link 경로로 들어옴. 프론트에는 그대로 emit.
- 활성 프로젝트 변경 시 watcher 재시작 X — workspace 가 바뀌지 않음.

### 사이드바 / UI

- 사이드바 "📁 프로젝트" 섹션은 그대로 (활성 전환). 새 항목 등록은 hub 자동 처리.
- `/explore` 도 그대로 — `docs_path` 가 link 경유로 바뀌었으므로 내부 변경 없음.
- 안내 문구 추가: 온보딩 화면 / 설정 페이지 상단에 "🏠 Hub: ~/Jiphyeon" 1줄 표시.

## Behavior Contract

### `default_workspace_path`

- Given: `$HOME=/Users/uno`
- When: 호출
- Then: `PathBuf::from("/Users/uno/Jiphyeon")`
- Given: HOME 미설정
- When: 호출
- Then: 현재 작업 디렉토리 기준 `./Jiphyeon` 폴백

### `ensure_workspace_dir`

- Given: 디렉토리 없음
- When: 호출
- Then: 생성됨 (`mkdir -p`)
- Given: 디렉토리 존재
- When: 호출
- Then: no-op `Ok(())`
- Given: 같은 경로에 일반 파일
- When: 호출
- Then: `Err(AppError::InvalidPath)`

### `create_project_symlink`

- Given: workspace 비어있음, target 유효
- When: 호출
- Then: `<workspace>/<name>` symlink 생성, `Ok(link_path)`
- Given: 동일 target 의 symlink 이미 있음
- When: 호출
- Then: 기존 link 재사용, 새로 만들지 않음 (idempotent)
- Given: `<workspace>/<name>` 에 다른 target 의 symlink 또는 일반 폴더가 있음
- When: 호출
- Then: `<name>-2` 시도, 다시 충돌이면 `-3`, ... 1000 회 시도 후 실패
- Given: target 미존재
- When: 호출
- Then: `Err(AppError::VaultNotFound)` 폴백

### `is_link_broken`

- Given: link 가 정상 symlink 이고 target 존재
- When: 호출
- Then: `false`
- Given: link 가 symlink 이지만 target 없음
- When: 호출
- Then: `true`
- Given: link 자체 미존재
- When: 호출
- Then: `true`

### `register_project` (변경 후)

- Given: target 신규 등록
- When: 호출
- Then: link 생성 + ProjectEntry.link_path 채움 + active_project_id 갱신
- Given: 동일 target 재등록
- When: 호출
- Then: 기존 link 재사용, projects vec 변경 없음, 활성만 전환 (현재와 동일)
- Given: 다른 target 이지만 같은 derived name
- When: 호출
- Then: `<name>-2` link 생성

### 마이그레이션 (`activate_existing_links`)

- Given: 시작 시 등록된 5 개 프로젝트 중 link 가 모두 없음
- When: setup
- Then: 5 개 모두 best-effort symlink 생성. 실패한 것은 `eprintln!` 로 로그만.

## Edge Cases

- `~/Jiphyeon` 이 사용자 직접 만든 일반 디렉토리 (symlink 들이 아닌) → 그대로 둠. 충돌 시 suffix 처리.
- 사용자가 hub 안에서 폴더를 직접 만듦 (등록 안 한) → 등록 안 됨, watcher 는 감시하지만 사이드바엔 안 보임.
- target 이 이동/삭제됨 → `is_link_broken` 으로 검출. 사이드바 표시는 v2.5+ 에서 (이번엔 기능만 노출).
- macOS APFS firmlink: hub 가 `~/Jiphyeon` 이라 `/System/Volumes/Data/...` 변환 가능 — `canonicalize` 는 안 함, 사용자 경로 그대로 보존.
- Windows: `std::os::windows::fs::symlink_dir` 은 권한 필요. 이번 슬라이스는 `#[cfg(unix)]` 로 한정. Windows 빌드 시 hub 기능 비활성 + 사용자에게 명시적 에러.

## Dependencies

- `dirs` 크레이트 추가 (홈 디렉토리 조회) — `dirs = "5"`
- `std::os::unix::fs::symlink` 사용 (Linux + macOS)
- 기존 `notify` watcher 인프라 그대로 (symlink follow 옵션만 활성)

## 비범위

- Hub 안에 "미등록 폴더" 표시 / 자동 등록
- broken link UI 처리
- workspace 위치 변경 (지금은 고정)
- Windows 지원
- graphify 실행 IPC

## 작업 순서 (TDD)

1. `dirs` 의존성 추가
2. `src-tauri/src/workspace.rs` 신규 — pure helpers + 단위 테스트 (TempDir 기반):
   - `default_workspace_path` HOME 환경
   - `ensure_workspace_dir` 신규/기존/파일충돌
   - `create_project_symlink` neutral/idempotent/충돌-suffix/target-missing
   - `is_link_broken` 3 케이스
3. `AppConfig` 에 `workspace_path` 추가 + load/save 테스트 갱신
4. `ProjectEntry` 에 `link_path` 추가 (기본 = root_path 폴백, deserialize default)
5. `commands/projects.rs::register_project` 가 symlink 만들도록 수정
6. `lib.rs::setup` 에 workspace 보장 + 기존 등록 마이그레이션
7. `watcher::start_watching` 시그니처는 그대로, 호출처가 active_project.root_path → workspace_path 로 교체
8. 프론트엔드: types.ts 의 `ProjectEntry` 에 `link_path: string`, `AppConfig` 에 `workspace_path: string` 추가
9. 사이드바 / settings 에 "🏠 Hub: ~/Jiphyeon" 표시 1 줄 추가
10. cargo test + clippy + svelte-check
11. 커밋 옵션:
    - 단일 커밋 (feat: workspace hub 도입 v2.5)
    - 두 커밋 (backend / frontend)

## 완료 조건

- 신규 register 시 `~/Jiphyeon/<name>` symlink 생성
- 기존 등록 5 개 → 다음 부팅 시 자동 마이그레이션
- watcher 가 workspace 전체 감시
- broken link 검출 가능 (UI 노출은 후속)
- macOS/Linux 빌드 통과, Windows 는 명시적 비활성
- Rust + clippy + svelte-check 0 errors
