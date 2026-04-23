import { invoke } from "@tauri-apps/api/core";
import type {
  RenderedNote,
  ClipRequest,
  ClipResult,
  AppConfig,
  AppConfigPatch,
  DetectedEditor,
  VaultEntry,
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

export interface VaultStatus {
  connected: boolean;
  vault_path: string | null;
}

export function getVaultStatus(): Promise<VaultStatus> {
  return invoke("get_vault_status");
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

export function listVaults(): Promise<VaultEntry[]> {
  return invoke("list_vaults");
}

export function switchVault(path: string): Promise<VaultStatus> {
  return invoke("switch_vault", { path });
}

export function removeVault(path: string): Promise<VaultEntry[]> {
  return invoke("remove_vault", { path });
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
