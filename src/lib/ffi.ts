// These are the raw FFI functions to the rust backend.
//
// You should avoid using these directly. Use the API functions instead.

import { invoke } from "@tauri-apps/api";

export interface Item {
  id: number;
  path: string;
  tags: string;
  meta_tags: string;
}

export async function openRepo(path: string) {
  await invoke("open_repo", { path: path });
}

export async function closeRepo() {
  await invoke("close_repo");
}

export enum ManagerStatus {
  IDLE = "Idle",
  SCANNING_DIRECTORY = "ScanningDirectory",
  UPDATING_REPO = "UpdatingRepo",
  // QUERYING = "Querying",
}

export async function getStatus(): Promise<ManagerStatus | null> {
  return await invoke("current_status");
}

export async function getRepoPath(): Promise<string | null> {
  return await invoke("current_path");
}

export async function queryItemIds(query: string): Promise<number[]> {
  return await invoke("query_item_ids", { query: query });
}

export async function getItem(id: number): Promise<Item> {
  return await invoke("get_item", { id: id });
}

export async function revealFile(path: string) {
  return await invoke("reveal_file", { path: path });
}

export async function openFile(path: string) {
  return await invoke("open_file", { path: path });
}

export enum FileType {
  AUDIO = "Audio",
  DOCUMENT = "Document",
  IMAGE = "Image",
  VIDEO = "Video",
  UNKNOWN = "Unknown",
}

export async function determineFileType(path: string): Promise<FileType> {
  return await invoke("determine_filetype", { path: path });
}
