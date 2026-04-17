## Spec: vault/indexer

### Public Interface

```rust
pub fn scan_vault(vault_path: &Path, exclude_dirs: &[String]) -> Result<VaultIndex, AppError>
pub fn build_backlinks(notes: &[NoteEntry]) -> HashMap<String, Vec<String>>
pub fn compute_stats(index: &VaultIndex) -> VaultStats
```

### Invariants

- `scan_vault`는 `*.md` 파일만 처리
- `exclude_dirs`에 포함된 디렉토리는 건너뜀
- 개별 파일 파싱 실패 시 해당 노트만 skip, 전체 스캔 중단 안 함
- backlinks 맵의 key는 노트 title (wikilink 대상), value는 참조하는 노트의 path 목록

### Behavior Contract

#### scan_vault

| Given | When | Then |
|-------|------|------|
| 볼트 디렉토리에 3개 .md 파일 존재 | `scan_vault` 호출 | `VaultIndex.notes`에 3개 `NoteEntry`, `scanned_at > 0` |
| `exclude_dirs`에 ".git" 포함, `.git/` 하위에 .md 파일 존재 | `scan_vault` 호출 | `.git` 하위 파일은 인덱스에 포함되지 않음 |
| 볼트에 .txt, .png 등 비-md 파일 존재 | `scan_vault` 호출 | `.md` 파일만 인덱스에 포함 |
| 파싱 실패하는 .md 파일 1개 + 정상 파일 2개 | `scan_vault` 호출 | 정상 파일 2개만 인덱스에 포함 (에러 전파 안 함) |
| 존재하지 않는 `vault_path` | `scan_vault` 호출 | `AppError::VaultNotFound` 반환 |
| 빈 볼트 디렉토리 | `scan_vault` 호출 | `VaultIndex { notes: [], backlinks: {}, scanned_at: now }` |

#### build_backlinks

| Given | When | Then |
|-------|------|------|
| note-a → `[[note-b]]`, note-c → `[[note-b]]` | `build_backlinks` 호출 | `backlinks["note-b"]` = [note-a 경로, note-c 경로] |
| 노트 간 링크 없음 | `build_backlinks` 호출 | 빈 `HashMap` 반환 |
| 동일 wikilink 중복 참조 | `build_backlinks` 호출 | backlinks에 중복 없이 한 번만 기록 |

#### compute_stats

| Given | When | Then |
|-------|------|------|
| til 2개, decision 1개, seedling 2개, growing 1개 | `compute_stats` 호출 | `by_type["til"]=2`, `by_type["decision"]=1`, `by_status["seedling"]=2`, `by_status["growing"]=1` |
| note-a → `[[note-b]]`, note-b → `[[note-c]]`, note-c는 링크 없음 | `compute_stats` 호출 | `orphan_notes` = 1 (note-a), `total_links` = 2 |
| note-a → `[[nonexistent]]` | `compute_stats` 호출 | `broken_links`에 "nonexistent" 포함 |

### Edge Cases

- 심볼릭 링크 → walkdir 기본 동작 따름 (follow하지 않음)
- frontmatter 없는 노트 → `note_type`, `status` 통계에서 제외
- 태그 집계 시 중복 제거 (`total_tags`는 고유 태그 수)

### Dependencies

- `vault::parser::parse_note` — 파일 파싱
- `walkdir` — 디렉토리 순회
- `chrono::Utc` — `scanned_at` 타임스탬프
