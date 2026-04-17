# Spec: folder-tree (Phase 2)

## Public Interface

### Backend

```rust
/// 볼트의 폴더 구조를 계층적 트리로 반환
#[tauri::command]
pub fn get_folder_tree(state: State<'_, VaultState>) -> Result<Vec<FolderNode>, AppError>
```

기존 시그니처 유지, 반환값의 `children`이 실제로 채워지도록 변경.

### 내부 헬퍼

```rust
/// 폴더 경로 목록에서 재귀 트리를 구축
fn build_tree(folder_counts: &HashMap<String, usize>) -> Vec<FolderNode>
```

### Frontend Component

```svelte
<!-- 재귀 폴더 트리 -->
<FolderTree
  nodes: FolderNode[]
  selectedPath?: string
  onSelect: (path: string) => void
  depth?: number  // 기본 0, 내부 재귀용
/>
```

## 기존 타입 (변경 없음)

```rust
pub struct FolderNode {
    pub name: String,          // 폴더명 (예: "dev")
    pub path: String,          // 상대 경로 (예: "dev/rust")
    pub note_count: usize,     // 이 폴더의 직접 노트 수 (하위 폴더 제외)
    pub children: Vec<FolderNode>,
}
```

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 빈 볼트 (노트 없음) | get_folder_tree | 빈 Vec 반환 |
| 2 | 루트에만 노트 있음 | get_folder_tree | `[FolderNode { name: ".", path: "", note_count: N, children: [] }]` |
| 3 | `dev/note.md`, `dev/rust/note.md` | get_folder_tree | `dev`(count=1) > `rust`(count=1) 계층 |
| 4 | 3단계 중첩 `a/b/c/note.md` | get_folder_tree | `a` > `b` > `c` 계층, 각 count 정확 |
| 5 | 여러 top-level 폴더 | get_folder_tree | 이름순 정렬 |
| 6 | 트리 노드 클릭 | FolderTree.onSelect | 해당 path로 필터 적용 |
| 7 | 자식 있는 노드 | FolderTree 렌더 | 접기/펼치기 토글 표시 |
| 8 | 자식 없는 노드 | FolderTree 렌더 | 토글 없이 폴더명만 표시 |

## Edge Cases

- 폴더에 노트 없고 하위 폴더에만 노트 있음 → `note_count: 0`, children은 채워짐
- 경로에 특수문자/한글 → `to_string_lossy()` 처리 (기존 패턴)
- 깊은 중첩(5단계+) → 제한 없음, UI에서 인덴트로 표현

## Dependencies

- 기존 `VaultIndex.notes`의 `path` 필드에서 폴더 구조 추출
- 신규 의존성 없음
- Mock boundary: 없음 (순수 계산)
