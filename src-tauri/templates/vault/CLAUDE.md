# Co-Vault

마크다운 기반 개인 지식 관리 볼트. Claude Code가 노트 생성/관리를 자동화한다.

## 볼트 구조

```
inbox/        빠른 캡처, 미분류, 웹 클리핑 저장
dev/          TIL, 기술 노트
decisions/    의사결정 기록 (ADR)
readings/     독서/아티클 메모
meetings/     회의록
ideas/        아이디어, 브레인스토밍
artifacts/    산출물, 다이어그램, 첨부파일
projects/     프로젝트별 하위 폴더
_moc/         Maps of Content (네비게이션 허브)
_templates/   노트 템플릿
_maintenance/ 감사/갭 분석 리포트 (자동 생성)
```

## 노트 타입 & 속성 스키마

모든 노트는 YAML frontmatter 필수.

| type | 폴더 | 필수 속성 | 선택 속성 |
|------|------|----------|----------|
| til | dev/ | type, created, tags | sources, status |
| decision | decisions/ | type, created, tags, status, context | alternatives, outcome |
| reading | readings/ | type, created, tags, sources | rating, status |
| meeting | meetings/ | type, created, tags, participants | action-items, project |
| idea | ideas/ | type, created, tags | status, priority |
| artifact | artifacts/ | type, created, tags, origin | format |
| clipping | inbox/ | type, created, tags, source | author, clipped, status |
| moc | _moc/ | type, created, tags | scope |

## Status 체계

| 값 | 의미 |
|----|------|
| seedling | 초기 메모, 미완성 |
| growing | 내용 보강 중 |
| evergreen | 완성, 주기적 리뷰 대상 |
| stale | 6개월 이상 미갱신, 리뷰 필요 |

## 링크 규칙

- `[[wikilink]]` 사용 (Markdown link 아님)
- 새 노트 작성 시 최소 1개 기존 노트와 링크
- MOC에서 해당 노트로의 진입점 확보
- 태그는 링크를 보완, 대체하지 않음

## 태그 컨벤션

- 계층 태그 사용: `#domain/backend`, `#domain/frontend`, `#project/booking`
- 상태 태그 금지 (frontmatter status 사용)
- 새 태그 생성 시 기존 태그 목록 확인 후 중복 방지

## 파일명 규칙

- 소문자 kebab-case: `kubernetes-pod-networking.md`
- 날짜 접두사 (meeting, decision, til, clipping): `2026-04-16-sprint-review.md`
- MOC는 PascalCase: `Home.md`, `Projects.md`

## Git 협업 규칙

- 세션 시작 시 `git pull --rebase` 실행 (최신 상태에서 작업)
- 커밋/push는 사용자가 명시적으로 요청할 때만 수행
- `_maintenance/reports/`는 `.gitattributes`에서 `merge=ours` — 충돌 시 로컬 유지, 각자 재생성

## Claude Code 작업 규칙

- 노트 생성/수정 후 frontmatter 유효성 확인
- 링크 생성 시 대상 노트 존재 여부 확인
- `_maintenance/reports/`에 분석 결과 저장
- 한국어로 작성 (기술 용어는 영어 허용)
- `_templates/`, `.claude/` 내부 파일은 일반 노트 분석에서 제외
