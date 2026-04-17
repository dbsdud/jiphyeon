---
name: vault-review
description: 주기적 리뷰 워크플로우 — 리뷰할 노트를 추천하고 볼트 현황을 요약한다
---

# 주기적 리뷰

볼트 현황을 요약하고 오늘 리뷰할 노트를 추천한다.
"/vault-review", "볼트 리뷰", "오늘 리뷰" 등의 키워드에 반응.

## 추천 기준 (우선순위 순)

1. **Stale 노트**: `status: stale`인 노트 (가장 오래된 순)
2. **방치된 seedling**: `status: seedling` + created 30일 이상 경과
3. **승격 직전**: `vault-mature` 승격 기준에 근접한 `growing` 노트
4. **랜덤 evergreen**: `status: evergreen` 중 무작위 1개 (간격 반복 리뷰)
5. **최근 빨간 링크**: 최근 작성된 노트에서 참조하지만 아직 없는 `[[wikilink]]` 대상

## 절차

1. Glob으로 전체 노트 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
2. 각 노트의 frontmatter + git 수정일 분석
3. 위 추천 기준으로 상위 5개 노트 선정
4. 볼트 현황 요약 출력:
   - 총 노트 수, 타입별 분포
   - status별 분포
   - 지난 7일간 생성/수정된 노트 수
5. "오늘 리뷰할 노트" 목록 출력 (각 노트에 추천 이유 포함)

## 출력 형식

```
## 볼트 현황
- 총 노트: N개 (til: X, decision: Y, ...)
- status: seedling X / growing Y / evergreen Z / stale W
- 최근 7일: 생성 N개, 수정 M개

## 오늘 리뷰할 노트
1. [[note-a]] — stale, 6개월간 미수정
2. [[note-b]] — seedling 45일 경과, 발전 또는 정리 필요
3. [[note-c]] — growing, 승격 기준 근접 (링크 1개 추가 필요)
4. [[note-d]] — evergreen 정기 리뷰
5. [[missing-note]] — 빨간 링크, 3개 노트에서 참조
```
