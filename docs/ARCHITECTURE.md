# Co-Vault Dashboard — 아키텍처

## 시스템 구조도

```
┌─────────────────────────────────────────────────────┐
│                    Tauri v2 App                      │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │              Svelte 5 Frontend                 │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐  │  │
│  │  │Dashboard │ │ Explorer │ │   Viewer     │  │  │
│  │  │Page      │ │ Page     │ │   Page       │  │  │
│  │  ├──────────┤ ├──────────┤ ├──────────────┤  │  │
│  │  │StatCards │ │NoteList  │ │MarkdownView │  │  │
│  │  │Charts   │ │FilterBar │ │BacklinkPanel│  │  │
│  │  │TagMap   │ │          │ │Frontmatter  │  │  │
│  │  │LinkGraph│ │          │ │Bar          │  │  │
│  │  └──────────┘ └──────────┘ └──────────────┘  │  │
│  │  ┌──────────────┐  ┌──────────────────────┐  │  │
│  │  │ WebClipper   │  │  Svelte Stores       │  │  │
│  │  │ Dialog       │  │  ($state runes)      │  │  │
│  │  └──────────────┘  └──────────────────────┘  │  │
│  └────────────────────┬──────────────────────────┘  │
│                       │ Tauri IPC (invoke/listen)    │
│  ┌────────────────────▼──────────────────────────┐  │
│  │              Rust Backend                      │  │
│  │  ┌────────────┐ ┌────────────┐ ┌───────────┐ │  │
│  │  │ vault      │ │ watcher    │ │ clipper   │ │  │
│  │  │ ┌────────┐ │ │            │ │           │ │  │
│  │  │ │parser  │ │ │ notify v6  │ │ reqwest   │ │  │
│  │  │ │indexer │ │ │ debounce   │ │ scraper   │ │  │
│  │  │ │graph   │ │ │ event emit │ │ html2md   │ │  │
│  │  │ └────────┘ │ │            │ │           │ │  │
│  │  └────────────┘ └────────────┘ └───────────┘ │  │
│  └───────────────────────────────────────────────┘  │
│                       │                              │
└───────────────────────┼──────────────────────────────┘
                        │ File I/O
              ┌─────────▼──────────┐
              │   know-vault/      │
              │   *.md files       │
              └────────────────────┘
```

## 프로젝트 구조

```
dashboard/
├── PRD.md
├── ARCHITECTURE.md
├── package.json
├── vite.config.ts
├── svelte.config.js
├── tsconfig.json
├── tailwind.config.ts
├── index.html
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   └── src/
│       ├── main.rs                # Tauri 앱 진입점
│       ├── lib.rs                 # 모듈 선언, Tauri Builder setup
│       ├── models.rs              # 공유 데이터 모델
│       ├── config.rs              # 앱 설정
│       ├── error.rs               # 에러 타입
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── vault.rs           # 볼트 조회 커맨드
│       │   ├── note.rs            # 노트 조회/렌더링 커맨드
│       │   └── clipper.rs         # 웹 클리핑 커맨드
│       ├── vault/
│       │   ├── mod.rs
│       │   ├── parser.rs          # frontmatter 파싱, wikilink 추출
│       │   ├── indexer.rs         # 볼트 전체 스캔, 인메모리 인덱스
│       │   └── graph.rs           # 링크 그래프 구축
│       ├── watcher/
│       │   ├── mod.rs
│       │   └── debounce.rs        # notify 이벤트 디바운스
│       └── clipper/
│           ├── mod.rs
│           ├── fetcher.rs         # HTTP 요청
│           ├── extractor.rs       # HTML 본문 추출
│           └── converter.rs       # HTML → Markdown 변환
│
└── src/
    ├── main.ts                    # Svelte 앱 진입점
    ├── App.svelte                 # 라우팅, 레이아웃
    ├── app.css                    # Tailwind 진입점
    ├── lib/
    │   ├── types.ts               # TypeScript 타입 (Rust 모델 미러)
    │   ├── ipc.ts                 # Tauri invoke 래퍼
    │   ├── stores/
    │   │   ├── vault.svelte.ts    # 볼트 인덱스, 통계 ($state)
    │   │   └── ui.svelte.ts       # 선택 노트, 페이지, 필터 ($state)
    │   └── components/
    │       ├── layout/
    │       │   ├── Sidebar.svelte
    │       │   └── Layout.svelte
    │       ├── dashboard/
    │       │   ├── StatCards.svelte
    │       │   ├── TypeChart.svelte
    │       │   ├── StatusChart.svelte
    │       │   ├── TagHeatmap.svelte
    │       │   ├── RecentNotes.svelte
    │       │   └── AuditSummary.svelte
    │       ├── explorer/
    │       │   ├── NoteList.svelte
    │       │   └── FilterBar.svelte
    │       ├── viewer/
    │       │   ├── MarkdownView.svelte
    │       │   ├── FrontmatterBar.svelte
    │       │   └── BacklinkPanel.svelte
    │       ├── graph/
    │       │   └── LinkGraph.svelte
    │       └── clipper/
    │           └── WebClipDialog.svelte
    └── pages/
        ├── DashboardPage.svelte
        ├── ExplorerPage.svelte
        ├── ViewerPage.svelte
        └── GraphPage.svelte
```

## 데이터 모델

### Rust (src-tauri/src/models.rs)

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// -- 열거형 --

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Til,
    Decision,
    Reading,
    Meeting,
    Idea,
    Artifact,
    Clipping,
    Moc,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NoteStatus {
    Seedling,
    Growing,
    Evergreen,
    Stale,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

// -- Frontmatter --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(rename = "type")]
    pub note_type: NoteType,
    pub created: NaiveDate,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Option<NoteStatus>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// -- 볼트 인덱스 --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    pub path: String,             // 볼트 루트 기준 상대 경로
    pub title: String,            // 파일명에서 추출
    pub frontmatter: Option<Frontmatter>,
    pub outgoing_links: Vec<String>,  // [[wikilink]] 대상 목록
    pub modified_at: i64,         // Unix timestamp
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultIndex {
    pub notes: Vec<NoteEntry>,
    pub backlinks: HashMap<String, Vec<String>>,  // 대상 → 참조하는 노트 목록
    pub scanned_at: i64,
}

// -- 통계/분석 --

#[derive(Debug, Clone, Serialize)]
pub struct VaultStats {
    pub total_notes: usize,
    pub by_type: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub by_folder: HashMap<String, usize>,
    pub total_links: usize,
    pub total_tags: usize,
    pub orphan_notes: usize,
    pub broken_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub count: usize,
}

// -- 링크 그래프 --

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub link_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// -- 노트 렌더링 --

#[derive(Debug, Clone, Serialize)]
pub struct RenderedNote {
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub html: String,             // pulldown-cmark 렌더링 결과
    pub outgoing_links: Vec<String>,
    pub backlinks: Vec<BacklinkEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkEntry {
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub context: String,          // 링크가 포함된 주변 텍스트
}

// -- 웹 클리핑 --

#[derive(Debug, Clone, Deserialize)]
pub struct ClipRequest {
    pub url: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipResult {
    pub path: String,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

// -- 이벤트 --

#[derive(Debug, Clone, Serialize)]
pub struct VaultChangeEvent {
    pub kind: ChangeKind,
    pub path: String,
}

// -- 폴더 트리 --

#[derive(Debug, Clone, Serialize)]
pub struct FolderNode {
    pub name: String,
    pub path: String,
    pub note_count: usize,
    pub children: Vec<FolderNode>,
}
```

### TypeScript (src/lib/types.ts)

```typescript
export type NoteType =
  | "til" | "decision" | "reading" | "meeting"
  | "idea" | "artifact" | "clipping" | "moc" | "unknown";

export type NoteStatus =
  | "seedling" | "growing" | "evergreen" | "stale" | "unknown";

export interface Frontmatter {
  note_type: NoteType;
  created: string;
  tags: string[];
  status?: NoteStatus;
  extra: Record<string, unknown>;
}

export interface NoteEntry {
  path: string;
  title: string;
  frontmatter: Frontmatter | null;
  outgoing_links: string[];
  modified_at: number;
  size: number;
}

export interface VaultStats {
  total_notes: number;
  by_type: Record<string, number>;
  by_status: Record<string, number>;
  by_folder: Record<string, number>;
  total_links: number;
  total_tags: number;
  orphan_notes: number;
  broken_links: string[];
}

export interface TagInfo {
  name: string;
  count: number;
}

export interface GraphNode {
  id: string;
  title: string;
  note_type: NoteType | null;
  link_count: number;
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface LinkGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface RenderedNote {
  path: string;
  title: string;
  frontmatter: Frontmatter | null;
  html: string;
  outgoing_links: string[];
  backlinks: BacklinkEntry[];
}

export interface BacklinkEntry {
  path: string;
  title: string;
  note_type: NoteType | null;
  context: string;
}

export interface ClipRequest {
  url: string;
  tags?: string[];
}

export interface ClipResult {
  path: string;
  title: string;
  success: boolean;
  error?: string;
}

export interface VaultChangeEvent {
  kind: "created" | "modified" | "deleted" | "renamed";
  path: string;
}

export interface FolderNode {
  name: string;
  path: string;
  note_count: number;
  children: FolderNode[];
}

export interface NoteFilter {
  folder?: string;
  note_type?: NoteType;
  status?: NoteStatus;
  tag?: string;
  query?: string;
  sort_by?: "created" | "modified" | "title" | "type";
}
```

## IPC 프로토콜

Tauri v2 `invoke` 기반. 모든 응답은 JSON 직렬화.

### 볼트 조회 (commands/vault.rs)

| 커맨드 | 파라미터 | 반환 | 설명 |
|--------|---------|------|------|
| `get_vault_stats` | — | `VaultStats` | 볼트 전체 통계 |
| `get_note_list` | `{ folder?, note_type?, status?, tag?, query?, sort_by? }` | `Vec<NoteEntry>` | 필터/정렬 노트 목록 |
| `get_tag_list` | — | `Vec<TagInfo>` | 태그 + 빈도 |
| `get_link_graph` | — | `LinkGraph` | 전체 링크 그래프 |
| `get_recent_notes` | `{ limit: u32 }` | `Vec<NoteEntry>` | 최근 수정 노트 |
| `get_folder_tree` | — | `Vec<FolderNode>` | 폴더 트리 |
| `search_notes` | `{ query: string }` | `Vec<NoteEntry>` | 파일명/태그/제목 검색 |
| `rescan_vault` | — | `VaultStats` | 전체 재스캔 |

### 노트 조회 (commands/note.rs)

| 커맨드 | 파라미터 | 반환 | 설명 |
|--------|---------|------|------|
| `get_note` | `{ path: string }` | `RenderedNote` | 노트 렌더링 + 백링크 |
| `get_backlinks` | `{ path: string }` | `Vec<BacklinkEntry>` | 백링크만 조회 |
| `open_in_editor` | `{ path: string }` | `()` | 외부 에디터 실행 |

### 웹 클리핑 (commands/clipper.rs)

| 커맨드 | 파라미터 | 반환 | 설명 |
|--------|---------|------|------|
| `clip_url` | `ClipRequest` | `ClipResult` | URL → 마크다운 → inbox/ |

### Tauri 이벤트 (Backend → Frontend)

| 이벤트 | 페이로드 | 설명 |
|--------|---------|------|
| `vault-changed` | `VaultChangeEvent` | 파일 변경 감지 |
| `vault-reindexed` | `VaultStats` | 재인덱싱 완료 |
| `clip-progress` | `{ status: string }` | 클리핑 진행 상태 |

## 파일 감시 아키텍처

```
┌─────────────────────────┐
│    notify v6 Watcher    │
│   (RecommendedWatcher)  │
│   볼트 루트 재귀 감시     │
├─────────────────────────┤
│   필터                   │
│   - *.md 파일만          │
│   - exclude_dirs 제외    │
└───────────┬─────────────┘
            │ raw events
┌───────────▼─────────────┐
│   Debouncer (500ms)     │
│   같은 파일 이벤트 병합   │
└───────────┬─────────────┘
            │ debounced events
┌───────────▼─────────────┐
│   Incremental Updater   │
│   Created → 파싱 → 추가  │
│   Modified → 재파싱      │
│   Deleted → 인덱스 제거   │
│   backlinks 맵 재계산    │
└───────────┬─────────────┘
            │ VaultChangeEvent
┌───────────▼─────────────┐
│   AppHandle.emit()      │
│   "vault-changed"       │
│   → 프론트엔드 자동 갱신  │
└─────────────────────────┘
```

## 상태 관리

### Rust (Backend)

`VaultIndex`를 `Arc<RwLock<VaultIndex>>`로 관리, Tauri `manage()`로 등록.

```rust
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(VaultIndex::default())))
        .manage(AppConfig::load_or_default())
        .invoke_handler(tauri::generate_handler![
            commands::vault::get_vault_stats,
            commands::vault::get_note_list,
            commands::vault::get_tag_list,
            commands::vault::get_link_graph,
            commands::vault::get_recent_notes,
            commands::vault::get_folder_tree,
            commands::vault::search_notes,
            commands::vault::rescan_vault,
            commands::note::get_note,
            commands::note::get_backlinks,
            commands::note::open_in_editor,
            commands::clipper::clip_url,
        ])
        .setup(|app| {
            // 1. 볼트 초기 스캔
            // 2. 파일 워처 시작
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Svelte (Frontend)

Svelte 5 runes 기반 스토어. 외부 라이브러리 없이 `$state`와 `$derived`로 관리.

```typescript
// lib/stores/vault.svelte.ts
class VaultStore {
  stats = $state<VaultStats | null>(null);
  notes = $state<NoteEntry[]>([]);
  tags = $state<TagInfo[]>([]);
  graph = $state<LinkGraph | null>(null);
  isLoading = $state(true);

  async fetchStats() { ... }
  async fetchNotes(filter?: NoteFilter) { ... }
  async fetchTags() { ... }
  async fetchGraph() { ... }
  handleChange(event: VaultChangeEvent) { ... }
}

export const vaultStore = new VaultStore();

// lib/stores/ui.svelte.ts
class UiStore {
  selectedNotePath = $state<string | null>(null);
  currentPage = $state<"dashboard" | "explorer" | "viewer" | "graph">("dashboard");
  filters = $state<NoteFilter>({});

  selectNote(path: string) { ... }
  navigate(page: string) { ... }
}

export const uiStore = new UiStore();
```

프론트엔드에서 `listen("vault-changed", ...)` 으로 백엔드 이벤트를 수신하고,
변경된 노트만 `vaultStore`에 반영한다.

## 설정

### tauri.conf.json

```json
{
  "productName": "Co-Vault Dashboard",
  "identifier": "com.covault.dashboard",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Co-Vault Dashboard",
        "width": 1280,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600
      }
    ]
  }
}
```

### 앱 설정 (config.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 볼트 루트 경로 (기본: dashboard의 부모 디렉토리)
    /// MVP: 단일 볼트. 향후 멀티 워크스페이스 확장 시 Vec<WorkspaceConfig>으로 변경 예정
    pub vault_path: PathBuf,
    /// 파일 감시 디바운스 (ms)
    pub watch_debounce_ms: u64,     // 기본: 500
    /// 최근 노트 표시 개수
    pub recent_notes_limit: usize,  // 기본: 20
    /// 제외 폴더
    pub exclude_dirs: Vec<String>,  // 기본: ["dashboard", ".git", "artifacts"]
    /// 외부 에디터 명령
    pub editor_command: String,     // 기본: "code"
}
```

## 기술 스택

| 영역 | 기술 | 이유 |
|------|------|------|
| 프레임워크 | Tauri v2 | 경량 WebView, Rust 백엔드 |
| 언어 (BE) | Rust | 파일 I/O 성능, Tauri 네이티브 |
| 언어 (FE) | TypeScript | 타입 안전성 |
| UI | Svelte 5 | 경량, 빌트인 반응성, 보일러플레이트 최소 |
| 빌드 | Vite | 빠른 HMR, Tauri 공식 지원 |
| 스타일 | Tailwind CSS 4 | 유틸리티 기반 |
| 상태관리 | Svelte stores ($state) | 빌트인, 외부 의존성 없음 |
| 차트 | layercake | Svelte 네이티브 차트 |
| 그래프 | d3-force + SVG | 직접 제어, 경량 |
| YAML 파싱 | serde_yaml | Serde 기반, 타입 안전 |
| 마크다운 | pulldown-cmark | 순수 Rust, 빠름 |
| 파일 탐색 | walkdir | 재귀 디렉토리 순회 |
| 파일 감시 | notify v6 | macOS FSEvents 네이티브 |
| HTTP | reqwest | 비동기 HTTP |
| HTML 파싱 | scraper | CSS 셀렉터 기반 |
| HTML→MD | html2md | 경량 변환 |
| 날짜 | chrono | Rust 날짜 표준 |
| 직렬화 | serde + serde_json | IPC 직렬화 |
