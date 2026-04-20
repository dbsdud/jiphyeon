export interface Frontmatter {
  note_type: string;
  created: string;
  tags: string[];
  status?: string;
}

export interface NoteEntry {
  path: string;
  title: string;
  frontmatter?: Frontmatter;
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

export interface GraphNode {
  id: string;
  path: string;
  title: string;
  note_type?: string;
  link_count: number;
  tags: string[];
}

export interface RecordingEntry {
  filename: string;
  size: number;
  modified_at: number;
  transcribed: boolean;
}

export interface GodNode {
  path: string;
  title: string;
  note_type?: string;
  backlink_count: number;
}

export interface ClusterInfo {
  id: number;
  size: number;
  representative_path: string;
  representative_title: string;
}

export interface ClusterSummary {
  cluster_count: number;
  largest_size: number;
  isolated_count: number;
  clusters: ClusterInfo[];
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

export interface SearchResult {
  path: string;
  title: string;
  frontmatter?: Frontmatter;
  modified_at: number;
  snippet?: string;
  match_field: string;
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

export interface ClaudeSkill {
  name: string;
  description: string;
  path: string;
}

export interface ClaudeHook {
  event: string;
  matcher: string | null;
  command: string;
  script_path: string | null;
}

export interface SkillWarning {
  path: string;
  reason: string;
}

export interface ClaudeTools {
  claude_md: string | null;
  skills: ClaudeSkill[];
  skill_warnings: SkillWarning[];
  hooks: ClaudeHook[];
  hooks_error: string | null;
}

export interface VaultEntry {
  path: string;
  name: string;
}

export type RescaffoldMode = "add-missing" | "force-claude";

export interface RescaffoldReport {
  created: string[];
  overwritten: string[];
  modified_by_user: string[];
  unchanged: number;
  dry_run: boolean;
}

export type NotificationLevel = "info" | "warn" | "error" | "success";

export interface NotificationEvent {
  level: NotificationLevel;
  message: string;
  source?: string | null;
  ts?: string | null;
}
