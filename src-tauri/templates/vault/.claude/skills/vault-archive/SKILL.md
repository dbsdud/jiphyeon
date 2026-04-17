---
name: vault-archive
description: stale 노트를 archive 폴더로 이동하여 볼트 노이즈를 줄인다
---

# 노트 아카이브

stale 노트나 더 이상 활성적이지 않은 노트를 아카이브로 이동한다.
"/vault-archive", "아카이브", "노트 정리" 등의 키워드에 반응.

## 아카이브 대상 후보

1. `status: stale`인 노트
2. `status: seedling` + created 90일 이상 경과 + 수정 없음
3. 사용자가 직접 지정한 노트

## 절차

1. **후보 탐색**: 위 기준으로 아카이브 후보 목록 생성
2. **영향 분석**: 각 후보 노트를 참조하는 다른 노트 목록 확인
   - incoming link가 있으면 경고 표시
3. **목록 출력**: 후보 노트를 이유와 함께 표시
4. **사용자 선택**: 사용자가 아카이브할 노트를 선택
5. **이동 실행**:
   - `archive/<원래폴더>/` 하위로 파일 이동 (예: `dev/old-note.md` → `archive/dev/old-note.md`)
   - 이동된 노트를 참조하던 `[[wikilink]]`에 `(archived)` 표시 추가
   - 관련 MOC에서 해당 노트 제거
6. **결과 보고**: 이동된 노트 수, 영향받은 링크 수 출력

## archive/ 폴더 구조

```
archive/
  dev/         아카이브된 dev 노트
  readings/    아카이브된 reading 노트
  ideas/       아카이브된 idea 노트
  ...
```

## 복원

사용자가 복원 요청 시:
1. `archive/` 에서 원래 폴더로 파일 이동
2. `(archived)` 표시 제거
3. MOC 재등록

## 규칙

- 아카이브는 삭제가 아님 — 파일은 보존되고 검색 가능
- vault-audit, vault-gap 등 분석 Skill에서 `archive/`는 기본 제외
- `archive/` 폴더는 필요 시 자동 생성
- decision 타입은 아카이브 비권장 (이력 보존 가치) — 경고 후 사용자 확인 필요
