# Spec: Project Files & `/explore` (Slice B-3)

**상태**: Draft
**작성일**: 2026-04-26
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice B-3
**선행**: Slice B-1, B-2

## 목표

활성 프로젝트의 `docs/` 디렉토리를 폴더 트리 + 파일 목록으로 탐색하는 `/explore` 페이지를 복원한다.
v1.0 의 `get_note_list(filters)` 같은 인덱서 의존 커맨드를 대체할 단순한 디렉토리 워크 커맨드 두 개를 추가한다.

## 배경

- Epic A-4 에서 `/explore` 가 placeholder 로 비워졌다.
- B-1 의 사이드바 프로젝트 스위처는 이미 적용됨. 이번 슬라이스는 **"파일 네비게이션 뷰"** 만 책임진다.
- 인덱서 없이 매 호출마다 디렉토리를 walk 해서 결과를 만든다. 캐싱 없음. 작은 docs/ (수백 파일) 가정.
- 태그 히트맵, 최근/타입/상태 필터, 검색은 모두 제외. 단순 폴더 트리 + 파일 목록 + 클릭 시 `/view` 이동.

## 데이터 모델

### Rust

```rust
// src-tauri/src/models.rs (추가)
pub struct ProjectFileEntry {
    pub path: String,         // docs_path 기준 상대 경로 (e.g. "decisions/2026-04-22.md")
    pub title: String,        // frontmatter 의 title 우선, 없으면 파일명(.md 제거)
    pub note_type: Option<String>,  // frontmatter 의 type 필드 (있으면)
    pub modified_at: i64,     // unix seconds
    pub size: u64,
}

pub struct ProjectFolderNode {
    pub name: String,
    pub path: String,         // docs_path 기준 상대 경로 ("" = root)
    pub note_count: usize,    // 직접 자식 파일 수 (재귀 X)
    pub children: Vec<ProjectFolderNode>,
}
```

기존 `models.rs::FolderNode`, `NoteEntry` 와 다른 새 타입을 둔다. 옛 타입은 다음 슬라이스/Epic 에서 제거.

### Frontend

```ts
export interface ProjectFileEntry {
  path: string;
  title: string;
  note_type: string | null;
  modified_at: number;
  size: number;
}

// FolderNode 는 v1.0 잔재였지만 모양이 동일. 재사용 — 단, FolderTree.svelte 가 의존하므로 유지.
```

## Public Interface

```rust
#[tauri::command]
pub fn list_project_files(
    config_state: State<'_, ConfigState>,
    subpath: Option<String>,
) -> Result<Vec<ProjectFileEntry>, AppError>;

#[tauri::command]
pub fn get_project_folder_tree(
    config_state: State<'_, ConfigState>,
) -> Result<ProjectFolderNode, AppError>;
```

```ts
export function listProjectFiles(subpath: string | null): Promise<ProjectFileEntry[]>;
export function getProjectFolderTree(): Promise<ProjectFolderNode>;
```

## Behavior Contract

### `list_project_files`

- Given: 활성 프로젝트 없음
- When: 호출
- Then: `Err(AppError::VaultNotConfigured)`
- Given: 활성 프로젝트, docs/ 미존재
- When: 호출
- Then: `Ok(vec![])` (빈 결과; 에러 X)
- Given: 활성 프로젝트, `subpath=None`
- When: 호출
- Then: docs/ 하위의 모든 `.md` 파일을 재귀로 수집해 반환. `modified_at` 내림차순 정렬.
- Given: 활성 프로젝트, `subpath=Some("decisions")`
- When: 호출
- Then: docs/decisions/ 하위 모든 `.md` 파일 (재귀) 반환
- 디렉토리 제외 규칙: `AppConfig.exclude_dirs` + `.git`/`.claude`/`node_modules`/`target` + dotfile 폴더
- Given: subpath 가 docs/ 바깥을 가리킴 (e.g. `"../etc"`)
- When: 호출
- Then: `Err(AppError::InvalidPath)` (path traversal 차단)
- 각 entry 의 `title`: 파일 첫 부분에서 frontmatter 파싱 시도 → 실패하면 파일 stem
- 각 entry 의 `note_type`: frontmatter 의 `type` 필드 (string)

### `get_project_folder_tree`

- Given: 활성 프로젝트 없음
- When: 호출
- Then: `Err(AppError::VaultNotConfigured)`
- Given: 활성 프로젝트, docs/ 미존재
- When: 호출
- Then: `Ok(ProjectFolderNode { name: "docs", path: "", note_count: 0, children: vec![] })`
- Given: 활성 프로젝트, docs/ 안에 파일과 하위 폴더 혼재
- When: 호출
- Then: 루트 노드 = docs (path=""). 하위 폴더는 알파벳 정렬. 각 노드의 `note_count` 는 그 폴더 직계 .md 파일 수.
- Given: 깊은 중첩
- When: 호출
- Then: 재귀적으로 전체 트리 반환 (제외 디렉토리: `AppConfig.exclude_dirs` + 점으로 시작하는 dotfile 폴더)
- Symlink: `is_dir()` 가 true 면 따라감 (경계 안전 검증은 하지 않음 — docs/ 안이 신뢰 영역이라 가정)

### Edge Cases

- 파일 첫 4 KiB 만 읽어 frontmatter 추출 (title 용). YAML 파싱 실패 시 fallback.
- 매우 큰 docs/ (1000+ 파일): 한 번 호출에 100ms 초과 시 advisor 호출 후 캐싱 도입 검토 — 이번 슬라이스에서는 단순 구현.
- exclude_dirs 는 `.git`, `.claude`, `node_modules`, `target` 기본. 디렉토리 이름 매칭만 사용 (경로 패턴 X).

## Frontend UI (`/explore`)

- 좌측 패널: `FolderTree.svelte` (이미 존재. props: `nodes: ProjectFolderNode[]`. 호환을 위해 wrapper 만들거나 직접 props 모양 맞춤).
  - 클릭한 폴더 path 는 상위 페이지가 보관 → `listProjectFiles(path)` 호출 → 우측 목록 갱신
- 우측 패널: 파일 목록. 클릭 시 `/view?path=...` 이동.
- v1.0 의 태그/타입/상태/검색 사이드바는 제거. 순수 트리 + 목록.
- 활성 프로젝트 변경 시 자동 재로드 (`vaultRefresh.version` track).

## Dependencies

- `vault::parser::extract_frontmatter` 재사용 (frontmatter 의 type/title 추출)
- 새 의존성 없음

## 비범위

- 검색 기능 (Epic E)
- frontmatter 의 status/tags 표시
- 파일 생성/이름변경/삭제 IPC
- watcher 와 explore 페이지 자동 갱신 연동 — 현재 `vaultRefresh.bump()` 만으로 처리. 별도 이벤트 fan-out 안 함.
- `models.rs` 에서 v1.0 잔재 `FolderNode`/`NoteEntry`/`Frontmatter`(rust) 정리 — Epic C 진입 시 일괄 처리

## 작업 순서 (TDD)

1. `models.rs` 에 `ProjectFileEntry`, `ProjectFolderNode` 추가 (serde Serialize 만)
2. `commands/projects.rs` 에 `list_project_files` 추가
   - 단위 테스트 (TempDir): 빈 docs / 파일만 / 폴더 + 파일 / subpath / path traversal 차단 / 활성 없음 → 5~6 케이스
   - frontmatter title/type 추출 케이스 1~2 개
3. `get_project_folder_tree` 추가
   - 단위 테스트: 빈 docs / 단일 레벨 / 중첩 / exclude_dirs 적용 / dotfile 폴더 제외
4. `lib.rs` invoke_handler 에 두 커맨드 추가
5. `api.ts` / `types.ts` 갱신
6. `/explore` 페이지 재작성 (`FolderTree.svelte` 재사용)
7. svelte-check + cargo test + clippy 통과
8. 커밋: `feat: /explore 폴더 트리 + 파일 목록 복원 (Slice B-3)`

## 완료 조건

- list_project_files / get_project_folder_tree 모든 BC 테스트 green
- `/explore` 가 활성 프로젝트의 docs/ 트리·파일 목록을 보여줌
- 파일 클릭 시 `/view?path=...` 로 이동, 활성 프로젝트 변경 시 자동 재로드
- Rust + clippy + svelte-check 전부 green
