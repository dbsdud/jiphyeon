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
  link_path: string;
  docs_path: string;
  graphify_out_path: string;
  registered_at: string;
  last_graphify_at: string | null;
}

export interface ProjectFileEntry {
  path: string;
  title: string;
  note_type: string | null;
  modified_at: number;
  size: number;
}

// --- graphify-out 도메인 (Slice C-3 부터) ---

export type GraphifyConfidence = "EXTRACTED" | "INFERRED" | "AMBIGUOUS" | "UNKNOWN";

export interface GraphifyNode {
  id: string;
  label: string;
  file_type: string | null;
  source_file: string | null;
  source_location: string | null;
  community: number | null;
  norm_label: string | null;
}

export interface GraphifyEdge {
  source: string;
  target: string;
  relation: string;
  confidence: GraphifyConfidence;
  confidence_score: number;
  source_file: string | null;
  weight: number;
}

export interface GraphifyHyperedge {
  id: string;
  label: string;
  nodes: string[];
  relation: string;
  confidence: GraphifyConfidence;
  confidence_score: number;
  source_file: string | null;
}

export interface GraphifyGraph {
  nodes: GraphifyNode[];
  edges: GraphifyEdge[];
  hyperedges: GraphifyHyperedge[];
}

export interface GraphReportSummary {
  nodes_count: number | null;
  edges_count: number | null;
  communities_count: number | null;
  extracted_pct: number | null;
  inferred_pct: number | null;
  ambiguous_pct: number | null;
  token_input: number | null;
  token_output: number | null;
}

export interface GraphReportGodNode {
  rank: number;
  name: string;
  edge_count: number;
}

export interface GraphReportSurprisingConnection {
  source: string;
  target: string;
  relation: string;
  confidence: GraphifyConfidence;
}

export interface GraphReportCommunity {
  id: number;
  label: string;
  cohesion: number | null;
  nodes_count: number | null;
  sample_nodes: string[];
}

export interface GraphReport {
  generated_at: string | null;
  project_root: string | null;
  summary: GraphReportSummary;
  god_nodes: GraphReportGodNode[];
  surprising_connections: GraphReportSurprisingConnection[];
  communities: GraphReportCommunity[];
}

export interface CrossProjectMember {
  project_id: string;
  project_name: string;
}

export interface CrossProjectNode {
  id: string;
  label: string;
  original_id: string;
  project_id: string;
  community: number | null;
  file_type: string | null;
  source_file: string | null;
  norm_label: string | null;
}

export interface CrossProjectEdge {
  source: string;
  target: string;
  relation: string;
  confidence: GraphifyConfidence;
  confidence_score: number;
  project_id: string | null;
  is_bridge: boolean;
}

export interface CrossProjectGraph {
  nodes: CrossProjectNode[];
  edges: CrossProjectEdge[];
  members: CrossProjectMember[];
}

export type PendingStatus = "fresh" | "stale" | "not_run" | "no_project";

export interface PendingGraphify {
  project_id: string | null;
  status: PendingStatus;
  graph_run_at: number | null;
  docs_changed_at: number | null;
  changed_files_count: number;
}

export interface GraphifyStatus {
  project_id: string | null;
  graphify_out_path: string | null;
  graph_json_exists: boolean;
  report_md_exists: boolean;
  last_run_at: string | null;
  nodes_count: number | null;
  edges_count: number | null;
}

export type ExplorerKind = "folder" | "file";

export interface ExplorerNode {
  kind: ExplorerKind;
  name: string;
  path: string;
  children: ExplorerNode[];
  note_type: string | null;
  modified_at: number | null;
}

export interface ProjectInspection {
  root_path: string;
  root_exists: boolean;
  docs_exists: boolean;
  docs_is_dir: boolean;
  graphify_out_exists: boolean;
  already_registered: boolean;
  suggested_name: string;
}

export interface AppConfig {
  workspace_path: string;
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
