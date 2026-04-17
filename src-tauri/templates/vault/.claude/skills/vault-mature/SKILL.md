---
name: vault-mature
description: 노트의 status를 자동 점검하여 승격/강등을 제안하고 적용한다
---

# 지식 성숙도 관리

노트의 status 생명주기를 자동으로 점검하고 전환을 제안한다.
"/vault-mature", "성숙도 점검", "status 정리" 등의 키워드에 반응.

## 전환 규칙

### 승격 후보
- `seedling` → `growing`: 본문이 500자 이상이고 `## 관련 노트`에 2개 이상 링크
- `growing` → `evergreen`: 본문 1000자 이상, 링크 3개 이상, 최소 1회 수정 이력(created와 git 마지막 수정일이 다름)

### 강등 후보
- `evergreen` → `stale`: git 마지막 수정일이 6개월 이상 경과
- `growing` → `stale`: git 마지막 수정일이 3개월 이상 경과

### 리마인드 대상
- `seedling`이면서 created가 30일 이상 경과: "발전시키거나 정리 필요"

## 절차

1. Glob으로 전체 `.md` 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`, `_moc/`)
2. 각 노트의 frontmatter에서 status, created 추출
3. `git log --format=%ai -1 -- <파일>` 로 마지막 수정일 확인
4. 본문 길이, 링크 수 측정
5. 전환 규칙에 따라 후보 분류 (승격/강등/리마인드)
6. 결과 요약 출력
7. 사용자 확인 후 선택된 노트의 frontmatter status 일괄 변경

## 규칙

- status 변경은 반드시 사용자 확인 후 적용
- 한 번에 전체 목록을 보여주고 선택적으로 적용 가능
- `_moc/` 노트는 status 관리 대상에서 제외
