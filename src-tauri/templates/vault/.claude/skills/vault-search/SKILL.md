---
name: vault-search
description: 태그, 타입, 날짜, 키워드 등 다양한 조건으로 볼트 노트를 검색한다
---

# 볼트 검색

자연어 또는 구조화된 조건으로 볼트 노트를 검색한다.
"/vault-search", "노트 검색", "노트 찾기" 등의 키워드에 반응.

## 지원 필터

| 필터 | 예시 | 설명 |
|------|------|------|
| type | `type:til` | 노트 타입 |
| tag | `tag:domain/backend` | 태그 (계층 포함) |
| status | `status:seedling` | status 값 |
| created | `created:2026-04` | 생성 날짜 (연, 연-월, 연-월-일) |
| modified | `modified:7d` | 최근 N일 내 수정 |
| keyword | `kubernetes networking` | 본문/제목 키워드 |
| folder | `folder:dev` | 특정 폴더 |
| link | `link:note-name` | 특정 노트를 참조하는 노트 |
| nolink | `nolink:true` | outgoing 링크가 없는 노트 |

## 절차

1. 사용자 쿼리를 파싱하여 필터 조합 결정
   - 자연어 입력: "지난주 작성한 backend TIL" → `type:til tag:domain/backend created:7d`
   - 구조화 입력: 필터를 직접 지정해도 됨
2. Glob으로 대상 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
3. 각 필터 순차 적용하여 결과 축소
4. modified 필터가 있으면 `git log`로 수정일 확인
5. 결과를 테이블 형식으로 출력

## 출력 형식

```
## 검색 결과 (N건)
| 파일 | 타입 | 태그 | 생성일 | status |
|------|------|------|--------|--------|
| [[note-a]] | til | #domain/backend | 2026-04-10 | growing |
| ... | ... | ... | ... | ... |
```

## 규칙

- 결과가 20건 초과 시 상위 20건만 출력 + 총 건수 안내
- 필터 조건 불명확 시 사용자에게 확인
- 50개 이상 파일 분석 필요 시 subagent에 위임
