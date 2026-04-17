---
name: vault-new
description: 새 노트를 생성하고 볼트 내 관련 노트를 자동으로 연결한다
---

# 새 노트 생성 + 자동 연결

사용자가 새 노트를 만들려고 할 때 이 skill을 사용한다.
"새 노트", "new note", "TIL", "메모", "기록" 등의 키워드에 반응.

## 절차

1. **타입 확인**: 사용자에게 노트 타입을 확인 (til, decision, reading, meeting, idea, artifact, clipping)
2. **파일 생성**: CLAUDE.md의 파일명 규칙에 따라 파일명 결정, 해당 타입의 템플릿(`_templates/tpl-<type>.md`) 내용을 기반으로 노트 생성
3. **관련 노트 검색**:
   - Grep으로 볼트 전체에서 주제 키워드 검색 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
   - 후보 노트의 frontmatter tags 비교
   - 상위 3~5개를 `## 관련 노트` 섹션에 `[[wikilink]]`로 추가
4. **MOC 연결**: 관련 MOC(`_moc/`)가 있으면 새 노트를 해당 MOC에 등록
5. **결과 보고**: 생성된 파일 경로, 연결된 노트 목록 출력

## 규칙

- frontmatter의 `created`는 오늘 날짜 (YYYY-MM-DD)
- 새 노트는 반드시 최소 1개 기존 노트와 링크 (볼트에 노트가 없는 초기에는 예외)
- 파일은 타입에 맞는 폴더에 저장
- 한국어로 작성 (기술 용어는 영어 허용)
