# 집현 (Jiphyeon)

> **集 賢** — *모을 집, 어질 현.* "현명함을 모으다."
>
> 1420년, 세종대왕은 집현전(集賢殿)을 세우고 이 땅의 학자들을 불러 모았다.
> 그들은 그곳에서 책을 읽고, 문답을 주고받고, 글을 엮었다. 훈민정음이 거기서 태어났다.
> 600년이 지나, 같은 이름의 공간이 다시 필요하다. 다만 이번에는 학자 자리에 **AI**가 앉는다.

**Jiphyeon**은 개인 지식을 체계적으로 축적하는 **LLM 위키 대시보드**다.
Tauri 기반의 데스크톱 앱이 볼트를 시각화하고, 볼트 안에서 실행되는 Claude Code가 집현전의 학자처럼 지식을 정리·편찬한다.

---

## 왜 LLM 위키인가

Andrej Karpathy가 제안한 [LLM 위키](https://github.com/karpathy/llm-wiki) 아이디어에서 출발했다.

> 사람들이 개인 위키를 포기하는 건 지식이 부족해서가 아니라,
> **유지 관리 비용이 가치보다 빠르게 증가하기 때문**이다.
>
> 교차 참조 업데이트, 요약 최신화, 수십 페이지에 걸친 일관성 유지.
> 이런 '북키핑(bookkeeping)'은 사람에겐 지루하지만, LLM은 지루함을 모른다.

Jiphyeon은 이 통찰을 구조화한다:

```
  사람  ────→ 무엇을 읽고, 무엇을 물을지 결정
    │
    ↓
  볼트  ────→ 마크다운 파일의 개인 지식 저장소
    │
    ↓
  AI   ────→ 읽고, 정리하고, 잇고, 모순을 지적
```

**사람은 사고한다. AI는 북키핑한다.** 그 경계를 명확히 나누는 것이 이 앱의 철학이다.

---

## 구조

Jiphyeon은 세 개의 층으로 움직인다.

| 층 | 역할 | 구현 |
|----|------|------|
| **앱 (대시보드)** | 볼트의 전체 상태를 한눈에. 통계, 링크 그래프, 마크다운 뷰어 | Tauri v2 + Svelte 5 + Rust |
| **볼트 (서고)** | 마크다운 파일, 템플릿, 태그, 위키링크 | 로컬 파일시스템 |
| **학자 (AI)** | 볼트 안에서 실행되는 Claude Code가 스킬과 훅으로 지식 관리 | `.claude/skills/`, `.claude/hooks/` |

앱은 **편집하지 않는다**. 보고, 찾고, 시각화할 뿐이다.
편집은 외부 에디터나 Claude Code가 수행한다. Obsidian이 IDE라면, Claude는 프로그래머, 볼트는 코드베이스다.

---

## 주요 기능

### 대시보드
- 노트 통계: 타입/상태별 분포, 태그 히트맵
- 링크 그래프: 볼트 전체의 wikilink 관계를 d3-force로 시각화
- 볼트 헬스: 고립 노트, 깨진 링크, 최근 활동
- 핵심 개념(God Node) 카드 *(v0.7 예정)*

### 마크다운 뷰어
- GitHub 스타일 렌더링 + GFM *(v0.6 예정)*
- 코드 하이라이팅, 수식(KaTeX), 다이어그램(Mermaid), DB 스키마(DBML) *(v0.6 예정)*
- wikilink 인터랙션, 백링크 패널

### 볼트 관리
- 멀티 볼트 등록/전환
- 볼트 스캐폴드 (14개 Claude 스킬 + 11개 훅 자동 설치)
- 파일 감시로 실시간 인덱스 갱신

### 소스 인제스트
- 웹 클리핑 (HTML → Markdown)
- 녹취/영상 전사 *(v0.8 예정)*
- URL에서 자막 추출 *(v0.8 예정)*

---

## 집현전 스킬들

볼트에 함께 설치되는 Claude 스킬은 집현전 학자들의 일과에 대응한다.

| 스킬 | 학자의 일 |
|------|---------|
| `vault-new` | 새 문헌 작성 |
| `vault-clip` | 외부 서책 필사 |
| `vault-link` | 서책 간 상호 참조 |
| `vault-mature` | 초고를 완성된 글로 다듬기 |
| `vault-synthesize` | 여러 문헌을 엮어 새 지식 편찬 |
| `vault-moc-sync` | 분야별 목차(MOC) 유지 |
| `vault-gap` | 비어 있는 지식 탐지 |
| `vault-audit` | 모순과 중복 점검 |
| `vault-archive` | 오래된 문헌 정리 |
| `vault-review` | 정기 복습 제안 |

---

## 기술 스택

- **Backend**: Rust (Tauri v2)
  - `pulldown-cmark` — 마크다운 파싱
  - `tantivy` — 전문 검색
  - `notify` — 파일 감시
  - `serde_yaml_ng` — YAML frontmatter
- **Frontend**: Svelte 5 (runes) + SvelteKit (adapter-static)
- **Styling**: Tailwind CSS 4
- **Visualization**: d3-force (링크 그래프), layercake (차트)

---

## 시작하기

```bash
# 개발 서버
npm install
npm run tauri dev

# 프로덕션 빌드
npm run tauri build

# Rust 테스트
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

첫 실행 시 온보딩 화면에서 볼트를 선택하거나 새로 생성할 수 있다.
새 볼트를 만들면 `.claude/` 디렉토리에 스킬·훅·CLAUDE.md가 함께 설치된다.

볼트 경로에서 `claude` 명령을 실행하면 집현전이 열린다.

---

## 로드맵

v1.0 MVP까지의 경로: [`docs/plans/v1.0-mvp-roadmap.md`](docs/plans/v1.0-mvp-roadmap.md)

| 버전 | 주제 |
|------|------|
| v0.5 (현재) | 에디터 연동 + Claude 도구 뷰 + 멀티 볼트 |
| v0.6 | UI 전면 개선 (디자인 시스템, GitHub 마크다운, 다이어그램) |
| v0.7 | 대시보드 강화 (God Node, 클러스터, 헬스 지표) |
| v0.8 | 녹취/영상 인제스트 (Whisper, YouTube) |
| v0.9 | 볼트 스킬·훅 정교화 + 온보딩 개선 |
| **v1.0** | **MVP 공개 배포** |

---

## 설계 문서

- [`docs/PRD.md`](docs/PRD.md) — 제품 요구사항
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — 아키텍처
- [`docs/specs/`](docs/specs/) — SDD 명세
- [`docs/designs/`](docs/designs/) — 설계 문서

---

## 철학

> 한 사람이 읽고 연결하고 이해하는 것은 여전히 사람의 일이다.
> 교차 참조를 업데이트하고 모순을 지적하고 파일을 일관성 있게 유지하는 것 — 그것이 AI의 일이다.
>
> 집현전은 학자들이 모여 **생각하는 곳**이었다.
> Jiphyeon도 그래야 한다.

---

## 후원

[![GitHub Sponsors](https://img.shields.io/github/sponsors/dbsdud?style=flat&logo=githubsponsors)](https://github.com/sponsors/dbsdud)

---

## 라이선스

MIT
