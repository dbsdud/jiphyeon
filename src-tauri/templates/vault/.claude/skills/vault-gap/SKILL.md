---
name: vault-gap
description: 볼트의 지식 영역 분포를 분석하여 부족한 영역과 트렌드를 리포트한다
---

# 지식 갭 분석

볼트의 지식 커버리지를 분석하고 부족한 영역을 식별한다.
"/vault-gap", "지식 갭", "부족한 영역" 등의 키워드에 반응.

## 분석 항목

### 1. 태그 분포 불균형
- 전체 태그 빈도 집계
- 계층 태그의 상위 카테고리별 비교 (예: `#domain/backend` 32건 vs `#domain/infra` 2건)
- 극단적 불균형 감지 (상위 카테고리 대비 10% 미만)

### 2. 노트 타입 분포
- 타입별 노트 수 집계
- 특정 타입 편중 감지 (예: TIL 90%, decision 0%)
- 기대 분포와 비교하여 부족한 타입 식별

### 3. 빨간 링크 (Red Links)
- `[[wikilink]]`로 참조되었지만 실제 파일이 없는 노트
- 빨간 링크가 많은 주제 = 작성이 필요한 지식

### 4. 프로젝트 커버리지
- `projects/` 하위 폴더별 노트 수
- 관련 decision, meeting 노트 유무
- 프로젝트 태그는 있지만 프로젝트 폴더/MOC가 없는 경우

### 5. 시간 트렌드
- 이전 gap 리포트(`_maintenance/reports/gap-*.md`)와 비교
- 개선/악화된 영역 식별

## 절차

1. Glob으로 전체 `.md` 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`)
2. 각 노트의 frontmatter에서 type, tags 추출
3. 본문에서 `[[wikilink]]` 추출하여 빨간 링크 식별
4. 위 5개 분석 항목 실행
5. 리포트를 `_maintenance/reports/gap-YYYY-MM-DD.md`에 저장
6. 구체적인 노트 작성 제안 출력 (예: "[[kubernetes-networking]] 노트 작성 추천 — 3개 노트에서 참조됨")

## 리포트 형식

```markdown
---
type: artifact
created: YYYY-MM-DD
tags:
  - maintenance/gap
origin: vault-gap
---
# 지식 갭 리포트 — YYYY-MM-DD

## 요약
- 총 노트: N개
- 태그 수: N개
- 빨간 링크: N개

## 태그 분포
| 카테고리 | 노트 수 | 비율 |
|---------|---------|------|
| ... | ... | ... |

## 부족한 영역
- ...

## 작성 추천 노트
- [[note-name]]: 이유
...

## 이전 대비 변화
- ...
```
