import { invoke } from "@tauri-apps/api/core";
import type {
  RenderedNote,
  ClipRequest,
  ClipResult,
  AppConfig,
  AppConfigPatch,
  CrossProjectGraph,
  DetectedEditor,
  ExplorerNode,
  FolderNode,
  GraphifyGraph,
  GraphifyStatus,
  GraphReport,
  ProjectEntry,
  ProjectFileEntry,
  ProjectInspection,
  RecordingEntry,
} from "./types";

export function getNote(path: string): Promise<RenderedNote> {
  return invoke("get_note", { path });
}

export function openInEditor(path: string): Promise<void> {
  return invoke("open_in_editor", { path });
}

export function createQuickNote(
  title: string | null,
  content: string,
  tags: string[],
  projectId: string | null = null,
): Promise<string> {
  return invoke("create_quick_note", { title, content, tags, projectId });
}

export function clipUrl(
  request: ClipRequest,
  projectId: string | null = null,
): Promise<ClipResult> {
  return invoke("clip_url", { request, projectId });
}

export function listProjects(): Promise<ProjectEntry[]> {
  return invoke("list_projects");
}

export function getActiveProject(): Promise<ProjectEntry | null> {
  return invoke("get_active_project");
}

export function inspectProjectRoot(rootPath: string): Promise<ProjectInspection> {
  return invoke("inspect_project_root", { rootPath });
}

export function listProjectFiles(subpath: string | null): Promise<ProjectFileEntry[]> {
  return invoke("list_project_files", { subpath });
}

export function getProjectFolderTree(): Promise<FolderNode> {
  return invoke("get_project_folder_tree");
}

export function getProjectExplorerTree(): Promise<ExplorerNode> {
  return invoke("get_project_explorer_tree");
}

export function getGraphifyGraph(): Promise<GraphifyGraph> {
  return invoke("get_graphify_graph");
}

export function getGraphifyReport(): Promise<GraphReport> {
  return invoke("get_graphify_report");
}

export function getGraphifyStatus(): Promise<GraphifyStatus> {
  return invoke("get_graphify_status");
}

export function getCrossProjectGraph(
  projectIds: string[],
  mergeLabels: boolean,
): Promise<CrossProjectGraph> {
  return invoke("get_cross_project_graph", {
    projectIds,
    mergeLabels,
  });
}

export function registerProject(
  rootPath: string,
  name: string | null,
  createDocs: boolean,
): Promise<ProjectEntry> {
  return invoke("register_project", {
    rootPath,
    name,
    createDocs,
  });
}

export function switchProject(id: string): Promise<ProjectEntry> {
  return invoke("switch_project", { id });
}

export function removeProject(id: string): Promise<ProjectEntry[]> {
  return invoke("remove_project", { id });
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

export function saveRecording(
  filename: string,
  bytes: Uint8Array,
  projectId: string | null = null,
): Promise<string> {
  return invoke("save_recording", { filename, bytes: Array.from(bytes), projectId });
}

export function deleteRecording(
  filename: string,
  projectId: string | null = null,
): Promise<void> {
  return invoke("delete_recording", { filename, projectId });
}

export function listRecordings(projectId: string | null = null): Promise<RecordingEntry[]> {
  return invoke("list_recordings", { projectId });
}

export function openCaptureWindow(): Promise<void> {
  return invoke("open_capture_window");
}
