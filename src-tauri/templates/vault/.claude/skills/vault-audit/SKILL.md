---
name: vault-audit
description: 볼트의 노트 품질을 감사하여 고아 노트, stale 노트, frontmatter 오류 등을 리포트한다
---

# 볼트 품질 감사

볼트 전체의 노트 품질을 점검하고 리포트를 생성한다.
"/vault-audit", "볼트 점검", "노트 점검" 등의 키워드에 반응.

## 점검 항목

### 1. 고아 노트 (Orphaned)
- 다른 노트에서 `[[wikilink]]`로 참조되지 않는 노트
- MOC에서도 도달할 수 없는 노트
- 심각도: 중

### 2. Stale 노트
- `status: seedling`이면서 `created`가 30일 이상 지난 노트
- `status: evergreen`이면서 수정일이 6개월 이상 지난 노트
- 심각도: 낮

### 3. Frontmatter 오류
- 필수 속성 누락 (CLAUDE.md 스키마 기준)
- `type` 값이 정의된 타입 목록에 없음
- `created` 형식이 YYYY-MM-DD가 아님
- 심각도: 높

### 4. 빈 섹션
- 템플릿에서 가져온 섹션이 비어있는 노트 (예: `## 핵심 내용` 아래 내용 없음)
- 심각도: 낮

### 5. 태그 위생
- 유사하지만 다른 태그 (예: `#backend` vs `#domain/backend`)
- 사용 빈도 1인 태그 (오타 가능성)
- 심각도: 중

### 6. 깨진 링크
- `[[wikilink]]` 대상 파일이 존재하지 않는 경우
- 심각도: 높

## 절차

1. Glob으로 전체 `.md` 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`)
2. 각 노트의 frontmatter와 본문 파싱
3. 위 6개 항목 순서대로 점검
4. 리포트를 `_maintenance/reports/audit-YYYY-MM-DD.md`에 저장
5. 터미널에 요약 출력: 항목별 발견 건수 + 심각도 높은 항목 하이라이트

## 리포트 형식

```markdown
---
type: artifact
created: YYYY-MM-DD
tags:
  - maintenance/audit
origin: vault-audit
---
# 볼트 감사 리포트 — YYYY-MM-DD

## 요약
| 항목 | 건수 | 심각도 |
|------|------|--------|
| ... | ... | ... |

## 상세
### 고아 노트
- [[note-name]]: incoming link 0개
...
```
