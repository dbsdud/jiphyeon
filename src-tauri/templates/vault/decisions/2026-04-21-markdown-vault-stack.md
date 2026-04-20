---
type: decision
created: 2026-04-21
tags:
  - architecture
  - pkm
status: evergreen
context: >-
  개인 지식 베이스의 포맷과 도구 선택. 글쓰기 경험·LLM 친화성·미래 포터빌리티를 동시에 만족해야 함.
outcome: 마크다운 파일 + wikilink + Git 버전 관리로 결정
---
# Markdown Vault Stack

## 컨텍스트

LLM 시대의 PKM을 만들면서, 어떤 **파일 포맷과 저장소 전략**을 쓸지 결정해야 했다. 주요 요구사항:

- 글쓰기 마찰이 작을 것
- LLM이 도구 없이 파싱할 수 있을 것
- 10년 뒤에도 읽을 수 있을 것 (포맷 포터빌리티)
- 여러 기기/협업 가능성

## 대안

| 옵션 | 장점 | 단점 |
|------|------|------|
| Notion | 에디터 UX, 임베드 | vendor lock-in, LLM이 API로만 접근 |
| Obsidian vault(markdown) | 로컬 파일, 플러그인 생태계 | 모바일 sync 복잡 |
| Roam / Logseq | 블록 기반, outliner | 포맷 복잡, 이탈 어려움 |
| **마크다운 + wikilink + Git** | 플랫 파일, LLM 친화, 포터블 | 에디터 UX는 스스로 만들어야 |

## 결정

**마크다운 파일 + `[[wikilink]]` + Git**로 결정. 뷰어는 `Jiphyeon`이 담당, 편집은 외부 에디터 또는 Claude Code.

## 근거

- LLM(Claude)이 파일을 직접 읽고 수정할 수 있어 북키핑 자동화가 자연스럽다.
- 10년 뒤에도 `grep`, `ripgrep`, `find`가 동작한다.
- Git 충돌 해결이 블록 기반 포맷보다 쉽다.

## 관련 노트

- [[building-a-second-brain]] — CODE 중 Express 단계를 어디서 할지의 문제와 연결
- [[seedling-llm-wiki]] — 이 결정이 LLM 위키 구현의 전제가 된다
- [[rust-ownership]] — Rust + Tauri로 앱을 구현한 이유는 정적 바이너리 배포 + 메모리 안전
