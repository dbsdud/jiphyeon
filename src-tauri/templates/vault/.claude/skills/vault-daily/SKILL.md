---
name: vault-daily
description: 일일 노트를 생성하여 오늘의 활동을 기록하고 관련 노트를 연결한다
---

# 일일 노트

오늘의 일일 노트를 생성하고 최근 활동을 연결한다.
"/vault-daily", "오늘 노트", "데일리", "일일" 등의 키워드에 반응.

## 절차

1. **파일 확인**: `inbox/YYYY-MM-DD-daily.md` 파일이 이미 있는지 확인
   - 있으면: 기존 파일 열어서 내용 추가
   - 없으면: 새로 생성
2. **어제 요약 수집**:
   - `git log --since="yesterday" --until="today"` 로 어제 수정된 노트 파악
   - 어제의 일일 노트가 있으면 action items 확인
3. **오늘의 맥락 구성**:
   - 오늘 생성/수정된 노트 목록
   - 진행 중인 seedling/growing 노트 중 최근 활동 있는 것
   - 오늘 날짜의 meeting 노트가 있으면 연결
4. **일일 노트 생성/갱신**

## 노트 구조

```markdown
---
type: til
created: YYYY-MM-DD
tags:
  - daily
status: seedling
---

# YYYY-MM-DD 일일 노트

## 어제 이어서
- [[yesterday-note]]: 진행 상황
- ...

## 오늘 작업
- (사용자가 기록)

## 배운 것
- (사용자가 기록)

## 내일 할 것
- (사용자가 기록)

## 오늘 관련 노트
- [[note-created-today]]
- [[note-modified-today]]
```

## 규칙

- 폴더: `inbox/` (일일 노트는 캡처 성격)
- 사용자가 내용을 채울 섹션은 빈 상태로 두고 placeholder 없이 유지
- 이미 있는 일일 노트에 추가할 때는 기존 내용 보존
- 어제 일일 노트의 "내일 할 것"이 있으면 "어제 이어서"로 자동 이관
