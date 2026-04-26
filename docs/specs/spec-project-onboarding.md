# Spec: Project Onboarding UI (Slice B-2)

**상태**: Draft
**작성일**: 2026-04-26
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice B-2
**선행**: Slice B-1 (`spec-project-registry.md`)

## 목표

활성 프로젝트가 없을 때 표시되는 온보딩 화면을 재작성한다.
사용자는 폴더를 선택하고 docs/ 존재 여부를 확인한 뒤 **명시적으로** docs/ 생성 여부를 결정한다.
graphify-out/ 부재 시 안내만 표시 (집현은 graphify를 직접 실행하지 않음).

## 배경

- Slice B-1 의 임시 stub `VaultOnboarding.svelte` 는 무조건 `create_docs=true` 로 호출 → docs/ 가 자동 생성됨.
- B-1 에 추가된 backend 분기는 이미 `register_project(create_docs: bool)` 로 양 쪽을 지원. 단, "docs/ 미존재" 응답을 프론트가 분기하지 못해 자동 생성으로만 동작 중.
- B-2 에서는 (a) docs/ 사전 점검을 backend 신규 커맨드로 분리하고 (b) UI 가 그 응답에 따라 다이얼로그를 띄워 사용자 의사를 묻도록 한다.

## Public Interface

### Tauri 커맨드 신규

```rust
#[tauri::command]
pub fn inspect_project_root(root_path: String) -> Result<ProjectInspection, AppError>;
```

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInspection {
    pub root_path: String,           // 정규화된 절대 경로
    pub root_exists: bool,           // false 면 등록 불가
    pub docs_exists: bool,
    pub docs_is_dir: bool,           // docs/ 가 파일이면 false
    pub graphify_out_exists: bool,
    pub already_registered: bool,    // 동일 root 가 이미 레지스트리에 있음
    pub suggested_name: String,      // derive_project_name 결과
}
```

### Frontend API

```ts
export interface ProjectInspection {
  root_path: string;
  root_exists: boolean;
  docs_exists: boolean;
  docs_is_dir: boolean;
  graphify_out_exists: boolean;
  already_registered: boolean;
  suggested_name: string;
}

export function inspectProjectRoot(rootPath: string): Promise<ProjectInspection>;
```

### 컴포넌트

- `ProjectOnboarding.svelte` 신규 (`VaultOnboarding.svelte` 대체).
- `AddVaultModal.svelte` → `AddProjectModal.svelte` 로 리네임 + 동일 플로우 재사용.
- 둘 다 내부적으로 동일한 "폴더 선택 → inspect → 결정 → register" 머신을 사용.

## Behavior Contract

### `inspect_project_root`

- Given: 존재하지 않는 경로
- When: 호출
- Then: `ProjectInspection { root_exists: false, ... }` (에러 X). 프론트가 사용자 메시지를 결정.
- Given: 존재하는 폴더, docs/ 없음, graphify-out/ 없음
- When: 호출
- Then: `docs_exists=false, docs_is_dir=false, graphify_out_exists=false`
- Given: 존재하는 폴더, docs/ 가 파일
- When: 호출
- Then: `docs_exists=true, docs_is_dir=false`
- Given: 존재하는 폴더, docs/ + graphify-out/ 모두 디렉토리
- When: 호출
- Then: 모두 true
- Given: 동일 root 가 이미 등록됨
- When: 호출
- Then: `already_registered=true`
- 모든 케이스: `suggested_name = derive_project_name(normalize_root)`, `root_path` 는 정규화된 문자열

### Onboarding UI 상태 머신

```
idle → picking → inspecting → decision → registering → done
                       ↓             ↓           ↓
                    error         error       error
```

- **idle**: 초기 화면. CTA "프로젝트 폴더 선택"
- **picking**: `openDialog({ directory: true })` 대기
- **inspecting**: `inspect_project_root(picked)` 대기
- **decision**: inspect 결과로 분기
  - `root_exists=false` → 에러, idle 로 복귀
  - `docs_exists=true, docs_is_dir=false` → 에러 ("docs 가 폴더가 아닙니다"), idle 로 복귀
  - `docs_exists=true` → 자동으로 register (`create_docs: false`)
  - `docs_exists=false` → 다이얼로그: "docs/ 폴더를 생성하시겠습니까?" / [생성 후 등록] [취소]
- **registering**: `register_project(root, name, create_docs)` 대기
- **done**: `onconnected(entry)` 콜백 → 부모가 화면 전환

### graphify-out 안내

- inspect 결과 `graphify_out_exists=false` 일 때 onboarding 카드 하단에 회색 보조 텍스트:
  > graphify-out/ 이 없습니다. 프로젝트를 Claude Code 에서 열고 `/graphify` 를 먼저 실행하세요.
- 등록 자체는 막지 않음. graphify-out 은 사용자 의도와 별개로 나중에 생길 수 있음.

### `already_registered` 처리

- inspect 응답에 포함만 하고, 다이얼로그에는 아래 메시지로 활용:
  > 이미 등록된 프로젝트입니다. 활성 프로젝트로 전환만 됩니다.
- register 호출은 그대로 진행 → backend 가 idempotent (B-1 동작) → 활성만 전환.

## Edge Cases

- 사용자가 폴더 선택을 취소 → idle 로 즉시 복귀, 에러 표시 X.
- inspect 중 race: 컴포넌트가 unmount → 결과 무시 (svelte 5 effect 정리). 별도 abort 토큰은 두지 않음.
- docs/ 가 심볼릭 링크인 경우: `is_dir()` 는 symlink target 을 따라가므로 폴더 리졸브 시 true 처리. 의도된 동작.
- 권한 에러로 docs/ 생성 실패 → register 단계에서 `AppError::Io` → 사용자에게 표시.

## Dependencies

- 기존 `project::normalize_root`, `project::derive_project_name` 재사용.
- 기존 `register_project` 시그니처는 그대로. backend 는 `inspect_project_root` 추가만 하면 됨.
- Frontend: `@tauri-apps/plugin-dialog` 의 `open()` (이미 사용 중).

## 비범위

- "프로젝트 추가" 사이드바 모달 외의 진입점 추가 (e.g. 설정 페이지 내 등록 폼) → 안 함.
- 등록 후 git init 권유 모달 (`GitInitModal.svelte`) → v1.0 잔재. 이번 슬라이스에서 제거 검토. v2.0 의 프로젝트는 보통 이미 git 레포라 git init 의미가 적음. 현재는 호출 안 됨 — UI 진입점만 정리.
- 다중 프로젝트 일괄 등록 → 안 함.
- docs/ 외에 graphify-out/ 자동 생성 → 안 함 (graphify 가 직접 만듦).

## 작업 순서 (TDD)

1. `commands/projects.rs::inspect_project_root` 단위 테스트 (TempDir 기반):
   - 미존재 경로 / docs 없음 / docs 가 파일 / docs 디렉토리 / 이미 등록됨 / graphify-out 유무
2. 구현
3. `lib.rs::invoke_handler` 에 `inspect_project_root` 추가
4. Frontend `api.ts` 에 `inspectProjectRoot` 추가, `types.ts` 에 `ProjectInspection`
5. `ProjectOnboarding.svelte` 신규 (state machine 포함)
6. `AddProjectModal.svelte` 신규 (동일 머신 재사용 — 단순한 wrapper 또는 props 기반 분기)
7. `+layout.svelte`: import 와 컴포넌트 사용을 새 이름으로 교체
8. 구 `VaultOnboarding.svelte`, `AddVaultModal.svelte`, `GitInitModal.svelte` 삭제
9. `cargo test --lib` + svelte-check 통과
10. 커밋: `feat: docs/ 자동 감지 온보딩 UI (Slice B-2)`

## 완료 조건

- inspect 5개 BC 전부 green
- 신규 `ProjectOnboarding.svelte` 가 docs/ 미존재 시 다이얼로그를 띄움
- graphify-out/ 부재 시 안내 표시
- `cargo test --lib` 전체 통과, svelte-check 0 errors
