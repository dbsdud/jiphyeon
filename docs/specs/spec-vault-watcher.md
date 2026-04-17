# Spec: watcher

## Public Interface

```rust
/// 볼트 감시 시작. 반환된 VaultWatcher를 drop하면 감시 중지.
pub fn start_watching(
    app_handle: tauri::AppHandle,
    config: &AppConfig,
) -> Result<VaultWatcher, AppError>

/// 감시 핸들 (drop 시 자동 중지)
pub struct VaultWatcher { /* debouncer 소유 */ }
```

## 테스트 가능한 순수 함수

```rust
/// 경로가 감시 대상인지 판별 (*.md, *.html + exclude_dirs 필터)
fn should_watch(path: &Path, vault_root: &Path, exclude_dirs: &[String]) -> bool

/// 파일 존재 여부 + 인덱스 존재 여부로 ChangeKind 판별
fn classify_change(exists_on_disk: bool, exists_in_index: bool) -> Option<ChangeKind>
```

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `.md` 파일 경로 | should_watch | true |
| 2 | `.html` 파일 경로 | should_watch | true |
| 3 | `.txt` 파일 경로 | should_watch | false |
| 4 | exclude_dirs 내 `.md` 파일 | should_watch | false |
| 5 | 디스크 존재 + 인덱스 미존재 | classify_change | Created |
| 6 | 디스크 존재 + 인덱스 존재 | classify_change | Modified |
| 7 | 디스크 미존재 + 인덱스 존재 | classify_change | Deleted |
| 8 | 디스크 미존재 + 인덱스 미존재 | classify_change | None (무시) |

## Edge Cases

- 중첩 exclude_dir 경로 (e.g. `sub/.git/note.md`) → 제외
- vault_root 바깥 경로 → false

## Dependencies

- `notify-debouncer-mini` — 디바운싱된 이벤트 수신
- `tauri::AppHandle` — `emit("vault-changed", event)` (mock boundary)

## 통합 흐름

```
notify RecommendedWatcher (recursive)
  → debouncer-mini (watch_debounce_ms)
  → should_watch 필터 (*.md, *.html + exclude_dirs)
  → classify_change로 ChangeKind 판별
  → VaultChangeEvent 생성
  → AppHandle.emit("vault-changed", event)
```
