---
name: vault-graph
description: 노트 간 링크 구조를 분석하여 그래프 데이터로 export하고 클러스터를 식별한다
---

# 노트 그래프 분석

볼트의 노트 간 연결 구조를 분석하고 시각화 데이터를 생성한다.
"/vault-graph", "그래프 분석", "연결 구조" 등의 키워드에 반응.

## 절차

1. **노트 수집**: Glob으로 전체 `.md` 파일 수집 (제외: `_templates/`, `.obsidian/`, `.claude/`, `_maintenance/`)
2. **그래프 구축**:
   - 각 노트를 노드로, `[[wikilink]]`를 엣지로 추출
   - 노드 속성: type, tags, status, created
   - 엣지 방향: 단방향 (A가 B를 참조)
3. **분석**:
   - **허브 노트**: incoming link가 가장 많은 상위 10개
   - **고립 클러스터**: 서로만 연결되고 나머지와 단절된 노트 그룹
   - **다리 노트**: 제거하면 그래프가 분리되는 노트 (bridging nodes)
   - **밀도**: 전체 가능 엣지 대비 실제 엣지 비율
   - **평균 경로 길이**: Home.md에서 임의 노트까지 평균 홉 수
4. **Export**: `_maintenance/reports/graph-YYYY-MM-DD.json` 으로 저장
5. **요약 출력**

## Export 형식

```json
{
  "generated": "YYYY-MM-DD",
  "stats": {
    "nodes": 0,
    "edges": 0,
    "density": 0.0,
    "clusters": 0,
    "avg_path_length": 0.0
  },
  "nodes": [
    { "id": "note-name", "type": "til", "tags": ["domain/backend"], "status": "growing", "in_degree": 3, "out_degree": 2 }
  ],
  "edges": [
    { "source": "note-a", "target": "note-b" }
  ],
  "analysis": {
    "hubs": ["note-a", "note-b"],
    "isolated_clusters": [["note-x", "note-y"]],
    "bridges": ["note-z"]
  }
}
```

## 요약 출력 형식

```
## 그래프 요약
- 노드: N개, 엣지: M개, 밀도: X%
- 클러스터: K개 (고립: J개)

## 허브 노트 (Top 5)
1. [[note-a]] — incoming 12개
2. ...

## 고립 클러스터
- {[[note-x]], [[note-y]]}: 나머지와 연결 없음

## 다리 노트
- [[note-z]]: 제거 시 2개 그룹으로 분리
```

## 규칙

- 50개 이상 노트 분석 시 subagent에 위임
- MOC 노트는 허브 분석에서 별도 표시 (구조적 허브 vs 자연 허브 구분)
