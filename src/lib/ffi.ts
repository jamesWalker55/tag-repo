// These are the raw FFI functions to the rust backend.
//
// You should avoid using these directly. Use the API functions instead.

import { invoke } from "@tauri-apps/api";

export async function openRepo(path: string) {
  await invoke("open_repo", { path: path });
}

export async function closeRepo() {
  await invoke("close_repo");
}

export async function getStatus(): Promise<string | null> {
  return await invoke("current_status");
}

export async function getRepoPath(): Promise<string | null> {
  return await invoke("current_path");
}
