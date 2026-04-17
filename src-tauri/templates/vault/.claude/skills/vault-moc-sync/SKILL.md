---
name: vault-moc-sync
description: MOC(Maps of Content)를 볼트 현재 상태에 맞게 자동 갱신한다
---

# MOC 자동 갱신

MOC 파일들을 볼트의 현재 노트 상태에 맞게 동기화한다.
"/vault-moc-sync", "MOC 갱신", "MOC 정리" 등의 키워드에 반응.

## 대상 MOC

| MOC | 갱신 로직 |
|-----|----------|
| `_moc/Home.md` | 하위 MOC 링크 확인, 누락된 MOC 추가 |
| `_moc/Topics.md` | `#domain/*` 태그 기준 카테고리별 노트 목록 재생성 |
| `_moc/Projects.md` | `projects/` 폴더 + `#project/*` 태그 기준 프로젝트 목록 갱신 |
| `_moc/Timeline.md` | 최근 30일 내 생성된 노트를 날짜순으로 나열 |

## 절차

1. Glob으로 전체 노트 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
2. 각 노트의 frontmatter에서 type, tags, created 추출
3. MOC별 갱신 로직 실행:

### Topics.md 갱신
- `#domain/*` 태그의 상위 카테고리 추출
- 카테고리별 H2 헤딩 아래에 해당 노트 `[[wikilink]]` 나열
- 카테고리 없는 새 `#domain/*`이 발견되면 섹션 추가

### Projects.md 갱신
- `projects/` 폴더의 하위 디렉토리 탐색
- `#project/*` 태그를 가진 노트 그룹화
- 프로젝트별 관련 노트(decision, meeting 등) 링크 정리

### Timeline.md 갱신
- 최근 30일 내 created인 노트를 날짜 역순으로 나열
- 타입별 아이콘: 📝 til, 🔀 decision, 📖 reading, 👥 meeting, 💡 idea

### Home.md 갱신
- 하위 MOC(Topics, Projects, Timeline) 링크 확인
- 볼트 통계 업데이트 (총 노트 수, 최근 활동)

4. 변경 사항을 diff로 보여주고 사용자 확인 후 적용

## 규칙

- 사용자가 수동으로 추가한 내용은 보존 (자동 생성 섹션만 갱신)
- 자동 생성 영역은 `<!-- auto-generated -->` ~ `<!-- /auto-generated -->` 주석으로 구분
- MOC에 없는 노트가 발견되면 적절한 MOC에 추가 제안
