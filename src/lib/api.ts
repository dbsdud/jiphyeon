import { invoke } from "@tauri-apps/api/core";
import type {
  VaultStats,
  NoteEntry,
  TagInfo,
  LinkGraph,
  GodNode,
  ClusterSummary,
  FolderNode,
  RenderedNote,
  BacklinkEntry,
  ClipRequest,
  ClipResult,
  SearchResult,
  AppConfig,
  AppConfigPatch,
  DetectedEditor,
  ClaudeTools,
  VaultEntry,
  RescaffoldMode,
  RescaffoldReport,
} from "./types";

export function getVaultStats(): Promise<VaultStats> {
  return invoke("get_vault_stats");
}

export function getNoteList(filters?: {
  folder?: string;
  note_type?: string;
  status?: string;
  tag?: string;
  query?: string;
  sort_by?: string;
}): Promise<NoteEntry[]> {
  return invoke("get_note_list", filters ?? {});
}

export function getTagList(): Promise<TagInfo[]> {
  return invoke("get_tag_list");
}

export function getLinkGraph(): Promise<LinkGraph> {
  return invoke("get_link_graph");
}

export function getRecentNotes(limit?: number): Promise<NoteEntry[]> {
  return invoke("get_recent_notes", { limit });
}

export function getFolderTree(): Promise<FolderNode[]> {
  return invoke("get_folder_tree");
}

export function searchNotes(query: string): Promise<SearchResult[]> {
  return invoke("search_notes", { query });
}

export function rescanVault(): Promise<VaultStats> {
  return invoke("rescan_vault");
}

export function getOrphanNotes(): Promise<NoteEntry[]> {
  return invoke("get_orphan_notes");
}

export function getTopGodNodes(limit: number): Promise<GodNode[]> {
  return invoke("get_top_god_nodes", { limit });
}

export function getClusterSummary(): Promise<ClusterSummary> {
  return invoke("get_cluster_summary");
}

export function getNote(path: string): Promise<RenderedNote> {
  return invoke("get_note", { path });
}

export function getBacklinks(path: string): Promise<BacklinkEntry[]> {
  return invoke("get_backlinks", { path });
}

export function openInEditor(path: string): Promise<void> {
  return invoke("open_in_editor", { path });
}

export function createQuickNote(
  title: string | null,
  content: string,
  tags: string[],
): Promise<string> {
  return invoke("create_quick_note", { title, content, tags });
}

export function clipUrl(request: ClipRequest): Promise<ClipResult> {
  return invoke("clip_url", { request });
}

export interface VaultStatus {
  connected: boolean;
  vault_path: string | null;
}

export function getVaultStatus(): Promise<VaultStatus> {
  return invoke("get_vault_status");
}

export function createVault(path: string): Promise<VaultStatus> {
  return invoke("create_vault", { path });
}

export function connectVault(path: string): Promise<VaultStatus> {
  return invoke("connect_vault", { path });
}

export function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export function updateConfig(patch: AppConfigPatch): Promise<AppConfig> {
  return invoke("update_config", { patch });
}

export function detectEditors(): Promise<DetectedEditor[]> {
  return invoke("detect_editors");
}

export function getClaudeTools(): Promise<ClaudeTools> {
  return invoke("get_claude_tools");
}

export function listVaults(): Promise<VaultEntry[]> {
  return invoke("list_vaults");
}

export function switchVault(path: string): Promise<VaultStatus> {
  return invoke("switch_vault", { path });
}

export function removeVault(path: string): Promise<VaultEntry[]> {
  return invoke("remove_vault", { path });
}

export function rescaffoldActiveVault(
  mode: RescaffoldMode,
  dryRun: boolean,
): Promise<RescaffoldReport> {
  return invoke("rescaffold_active_vault", { mode, dryRun });
}

export function saveRecording(filename: string, bytes: Uint8Array): Promise<string> {
  return invoke("save_recording", { filename, bytes: Array.from(bytes) });
}

export function deleteRecording(filename: string): Promise<void> {
  return invoke("delete_recording", { filename });
}
