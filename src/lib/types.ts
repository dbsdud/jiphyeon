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

export interface ProjectEntry {
  id: string;
  name: string;
  root_path: string;
  docs_path: string;
  graphify_out_path: string;
  registered_at: string;
  last_graphify_at: string | null;
}

export interface AppConfig {
  projects: ProjectEntry[];
  active_project_id: string | null;
  watch_debounce_ms: number;
  exclude_dirs: string[];
  editor_command: string;
  global_shortcut: string;
  density: Density;
  theme: ThemePreference;
  sidebar_collapsed: boolean;
}

export interface AppConfigPatch {
  editor_command?: string;
  exclude_dirs?: string[];
  global_shortcut?: string;
  density?: Density;
  theme?: ThemePreference;
  sidebar_collapsed?: boolean;
}

export interface DetectedEditor {
  id: string;
  label: string;
  command: string;
}

export type NotificationLevel = "info" | "warn" | "error" | "success";

export interface NotificationEvent {
  level: NotificationLevel;
  message: string;
  source?: string | null;
  ts?: string | null;
}

// Epic B-3 (FolderTree)·C-5 (LinkGraph) 재활용 대기 타입.
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
