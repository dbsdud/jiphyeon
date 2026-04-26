import { invoke } from "@tauri-apps/api/core";
import type {
  RenderedNote,
  ClipRequest,
  ClipResult,
  AppConfig,
  AppConfigPatch,
  DetectedEditor,
  ExplorerNode,
  FolderNode,
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
): Promise<string> {
  return invoke("create_quick_note", { title, content, tags });
}

export function clipUrl(request: ClipRequest): Promise<ClipResult> {
  return invoke("clip_url", { request });
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

export function saveRecording(filename: string, bytes: Uint8Array): Promise<string> {
  return invoke("save_recording", { filename, bytes: Array.from(bytes) });
}

export function deleteRecording(filename: string): Promise<void> {
  return invoke("delete_recording", { filename });
}

export function listRecordings(): Promise<RecordingEntry[]> {
  return invoke("list_recordings");
}
