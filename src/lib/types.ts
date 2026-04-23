export interface Frontmatter {
  note_type: string;
  created: string;
  tags: string[];
  status?: string;
}

export interface BacklinkEntry {
  path: string;
  title: string;
  note_type?: string;
  context: string;
}

export interface RenderedNote {
  path: string;
  title: string;
  frontmatter?: Frontmatter;
  html: string;
  outgoing_links: string[];
  backlinks: BacklinkEntry[];
}

export interface RecordingEntry {
  filename: string;
  size: number;
  modified_at: number;
}

// Epic B-3 (FolderTree)·C-5 (LinkGraph) 재활용 대기 타입.
// Rust 백엔드의 대응 구조체는 Epic 진입 시 graphify 노드 기반으로 재작성 예정.
export interface GraphNode {
  id: string;
  path: string;
  title: string;
  note_type?: string;
  link_count: number;
  tags: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
}

export interface LinkGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface FolderNode {
  name: string;
  path: string;
  note_count: number;
  children: FolderNode[];
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

export type Density = "regular" | "compact";
export type ThemePreference = "light" | "dark" | "system";
/** resolved(실제 적용) 테마 — system은 OS 설정으로 계산된 값. */
export type ResolvedTheme = "light" | "dark";

export interface AppConfig {
  vault_path: string | null;
  watch_debounce_ms: number;
  recent_notes_limit: number;
  exclude_dirs: string[];
  editor_command: string;
  quick_note_folder: string;
  global_shortcut: string;
  density: Density;
  theme: ThemePreference;
  sidebar_collapsed: boolean;
}

export interface AppConfigPatch {
  editor_command?: string;
  exclude_dirs?: string[];
  recent_notes_limit?: number;
  global_shortcut?: string;
  quick_note_folder?: string;
  density?: Density;
  theme?: ThemePreference;
  sidebar_collapsed?: boolean;
}

export interface DetectedEditor {
  id: string;
  label: string;
  command: string;
}

export interface VaultEntry {
  path: string;
  name: string;
}

export type NotificationLevel = "info" | "warn" | "error" | "success";

export interface NotificationEvent {
  level: NotificationLevel;
  message: string;
  source?: string | null;
  ts?: string | null;
}
