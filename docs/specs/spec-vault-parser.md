## Spec: vault::parser

### Public Interface

```rust
pub fn parse_note(path: &Path, vault_root: &Path) -> Result<NoteEntry, AppError>
pub fn extract_frontmatter(content: &str) -> Option<Frontmatter>
pub fn extract_wikilinks(content: &str) -> Vec<String>
pub fn title_from_path(path: &Path) -> String
```

### Invariants
- `path`는 `.md` 파일이어야 함
- `NoteEntry.path`는 vault_root 기준 상대 경로 (예: `dev/my-note.md`)
- wikilink 추출은 `[[target]]` 형식, `[[target|alias]]`도 target만 추출
- frontmatter는 파일 시작의 `---` ~ `---` 블록

### Behavior Contract

- Given: `---\ntype: til\ncreated: 2026-04-16\ntags: [rust]\n---\n# Hello\n[[other-note]]`
  - When: parse_note
  - Then: NoteEntry { frontmatter: Some(Frontmatter { note_type: Til, ... }), outgoing_links: ["other-note"] }

- Given: frontmatter 없는 순수 마크다운
  - When: parse_note
  - Then: NoteEntry { frontmatter: None, title: 파일명에서 추출 }

- Given: 잘못된 YAML (예: `type: [invalid`)
  - When: parse_note
  - Then: NoteEntry { frontmatter: None } (에러 스킵, 전체 실패 아님)

- Given: `[[note-a]] some text [[note-b|alias]]`
  - When: extract_wikilinks
  - Then: ["note-a", "note-b"]

- Given: `2026-04-16-my-til.md`
  - When: title_from_path
  - Then: "2026-04-16-my-til"

### Edge Cases
- frontmatter `---` 가 본문에도 등장하는 경우 → 첫 번째 블록만 파싱
- 빈 파일 → frontmatter: None, outgoing_links: []
- tags 필드가 문자열인 경우 (YAML 단일값) → Vec<String>로 변환

### Dependencies
- `serde_yaml_ng`: YAML 파싱
- `regex`: wikilink 추출
- `chrono`: NaiveDate
- Mock boundary: 없음 (순수 파일 I/O)
