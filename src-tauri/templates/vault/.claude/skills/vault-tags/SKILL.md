---
name: vault-tags
description: 볼트의 태그를 조회, 이름 변경, 병합하여 태그 체계를 관리한다
---

# 태그 관리

볼트 전체의 태그를 조회하고 리팩토링한다.
"/vault-tags", "태그 정리", "태그 관리" 등의 키워드에 반응.

## 명령

### list (기본)
전체 태그 목록을 사용 빈도순으로 출력.

```
## 태그 목록 (N개)
| 태그 | 사용 횟수 | 노트 |
|------|----------|------|
| #domain/backend | 12 | note-a, note-b, ... |
| #project/booking | 5 | ... |
```

### find `<keyword>`
키워드가 포함된 태그 검색.

### rename `<old>` → `<new>`
태그 이름 변경. 해당 태그를 사용하는 모든 노트의 frontmatter를 일괄 수정.

### merge `<tag-a>` + `<tag-b>` → `<target>`
두 태그를 하나로 통합. 두 태그를 사용하는 모든 노트에서 target 태그로 교체.

### orphan
어떤 노트에서도 사용되지 않는 태그 탐지 (태그 정의 없이 노트에만 존재하므로, 사용 빈도 1인 태그를 오타 후보로 표시).

### tree
계층 태그를 트리 구조로 시각화.
```
domain/
  ├── backend (12)
  ├── frontend (8)
  └── infra (3)
project/
  ├── booking (5)
  └── payment (2)
```

## 절차

1. Glob으로 전체 `.md` 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
2. 각 노트의 frontmatter에서 tags 배열 추출
3. 명령에 따라 분석/변경 수행
4. rename/merge 시:
   - 변경 대상 노트 목록을 미리 보여주고 사용자 확인
   - 확인 후 일괄 수정
   - MOC에서 해당 태그 참조가 있으면 함께 수정

## 규칙

- rename/merge는 반드시 사용자 확인 후 적용
- 계층 태그 규칙 유지: 새 태그도 `#카테고리/하위` 형식 권장
- 변경 후 영향받은 노트 수 출력
