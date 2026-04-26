# Spec: Dashboard v2 (Slice C-6)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: Slice C-3

## 목표

`/` (Dashboard) 를 graphify 출력 기반 카드 모음으로 재작성. 활성 프로젝트의 `GraphifyStatus` + `GraphReport` 를 가져와 표시.

## 페이지 구성

```
┌──────────────────────────────────────────────────────┐
│ 📁 <project name>           Last graphify: 2 hours ago│
├──────────────────────────────────────────────────────┤
│ Summary 카드: nodes / edges / communities / 토큰비용   │
├──────────────────────────────────────────────────────┤
│ God Nodes (top 10)                                    │
│ 1. GET()         76 edges                             │
│ ...                                                   │
├──────────────────────────────────────────────────────┤
│ Surprising Connections (top 5, scroll for more)       │
│ runCycle() --calls--> releaseStuckLocks() [INFERRED]  │
├──────────────────────────────────────────────────────┤
│ Communities (size 내림차순, top 5)                    │
└──────────────────────────────────────────────────────┘
```

빈 상태:
- 활성 프로젝트 없음 → 온보딩 유도 (이미 layout 에서 처리)
- 활성 프로젝트 있음, `graph_json_exists=false` → 큰 안내 카드:
  > 이 프로젝트에서 graphify 가 실행되지 않았습니다.
  > 터미널에서 `cd ~/Jiphyeon/<name>` → Claude Code 실행 → `/graphify`

## 데이터 흐름

- 한 페이지 안에서 `getGraphifyStatus()` + `getActiveProject()` + (status.graph_json_exists 면) `getGraphifyReport()` 병렬 호출
- `vaultRefresh.version` 추적해 자동 재로드

## 컴포넌트 분리

- 카드들을 별도 컴포넌트로 빼지 않고 단일 `+page.svelte` 안에서 처리 (재사용 X, 단순함 우선)
- 카드 디자인 재활용은 컴포넌트 추출이 필요할 때 별도 슬라이스

## 액션

- God Nodes / Communities / Surprising 의 노드 이름 클릭 → 일단 동작 없음 (라벨 표시만). 후속에서 `/graph` 로 포커스 이동 추가.
- 빈 상태 카드의 "Claude Code 열기" 버튼 — 일단 안내 텍스트만, 실행은 v2.6+

## 작업 순서

1. `/+page.svelte` 재작성: status 분기 + report 카드들
2. (필요 시) helper: 시간 차이 사람 친화 표기 (`Last graphify: 2 hours ago`) — `Intl.RelativeTimeFormat` 사용
3. svelte-check + 커밋
