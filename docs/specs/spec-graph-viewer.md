# Spec: /graph Page Redesign (Slice C-5)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: Slice C-1 ~ C-3

## 목표

`/graph` 페이지를 graphify 출력 기반으로 재작성한다. 활성 프로젝트의 `GraphifyGraph` 를 가져와 기존 `LinkGraph.svelte` (d3-force) 위에 렌더링한다.

## 컴포넌트 변경

- `LinkGraph.svelte` 의 props 시그니처를 v1 `LinkGraph` → `GraphifyGraph` 로 교체.
  - 입력: nodes (id/label/community/file_type/source_file), edges (source/target/relation/confidence/confidence_score)
  - 노드 ID 는 그래프 안에서 유일해야 → graphify 의 id 그대로 사용
- 시각화 규칙
  - 노드 색상: `community` 기반 (d3 `schemeTableau10` 또는 12색 반복). community=null → 회색
  - 노드 크기: degree 기반 (frontend 계산, `Math.sqrt(degree+1)*3 + 4`)
  - 엣지 stroke: confidence 기반
    - EXTRACTED → solid `1px`
    - INFERRED → solid `0.6px`
    - AMBIGUOUS → dashed `0.6px`
    - UNKNOWN → dashed `0.4px`
  - 엣지 opacity: `confidence_score` (0.3~1.0 으로 클램프)
  - 노드 레이블: 줌 0.6 이상에서만 표시 (현재 동작 유지)
- 노드 클릭 → 새 콜백 prop `onSelect(node)`
  - graph viewer 페이지가 source_file 처리 (md → /view, code → 외부 에디터)

## /graph 페이지

- 활성 프로젝트 없음 → 안내 카드
- `getGraphifyStatus()` → graph_json_exists=false → "Claude Code 에서 `cd ~/Jiphyeon/<project>` 후 `/graphify` 실행" 안내
- graph 있음 → `getGraphifyGraph()` 로 풀 그래프 가져와 LinkGraph 렌더
- 상단 헤더: 노드/엣지/하이퍼엣지 카운트 + 마지막 실행 시각
- 우측 사이드 패널 (선택된 노드 정보, optional) — 이번 슬라이스에서는 간단히 노드 클릭 시 source_file 로 이동만 (md → /view, 그 외 → openInEditor 시도)
- 필터는 v1 시절 `graph-filter.ts` 가 있지만 v2 노드는 tags 가 없어 필터 호환 X. 일단 검색(label) + community 드롭다운만.

## 비범위

- 하이퍼엣지 시각화 (다각형 묶음) — 헤더 카운트만
- 크로스 프로젝트 그래프 → C-7
- 노드 상세 패널 (이웃 강조 등) → 후속

## 작업 순서

1. `LinkGraph.svelte` props 교체 + 색상/엣지 규칙 갱신, `graph-filter.ts` 의존 제거
2. `/graph/+page.svelte` 재작성 (status 분기 + 그래프 + 검색/community 필터)
3. svelte-check + 커밋

## C-5b. Canvas 전환 (성능)

bloghub (1166 노드 / 1934 엣지) 에서 SVG + Svelte reactivity 가 60fps 를 못 유지 — Canvas 로 렌더링 교체.

- d3-force 시뮬레이션 그대로 유지
- `<canvas>` + `requestAnimationFrame` 직접 그리기, Svelte reactivity 우회 (simNodes/simLinks 는 plain mutable)
- d3-zoom 으로 transform 핸들, ctx.translate/scale 적용
- 포인터 이벤트 직접 처리:
  - down/move/up 으로 노드 드래그 (hit-test = 거리 비교)
  - hover 시 floating label
  - 클릭 (드래그 X) 시 onSelect 콜백
- 줌 > 0.8 일 때 viewport 안 노드만 ctx.fillText
- 노드 800+ 시 alphaDecay 0.05 (빠른 안정화)
- DPR 인식 (디스플레이 해상도 맞춰 sharper)
- ResizeObserver 로 컨테이너 크기 변화 대응
