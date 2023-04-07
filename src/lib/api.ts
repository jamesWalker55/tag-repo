import { provide, reactive } from "vue";
import { pollUntilComplete } from "@/lib/utils";
import * as ffi from "@/lib/ffi";
import { open } from "@tauri-apps/api/dialog";
import { listen, Event } from "@tauri-apps/api/event";

interface AppState {
  path: string | null;
  status: string | null;
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: AppState = reactive({
  path: null,
  status: null,
});

// listen to change events from the backend
(async () => {
  await Promise.all([
    listen("item-added", (x: Event<string>) => {
      console.log("item-added", x);
    }),
    listen("item-removed", (x: Event<string>) => {
      console.log("item-removed", x);
    }),
    listen("item-renamed", (x: Event<[string, string]>) => {
      console.log("item-renamed", x);
    }),
    listen("status-changed", (x: Event<string>) => {
      state.status = x.payload;
    }),
    listen("repo-path-changed", (x: Event<string>) => {
      state.path = x.payload;
    }),
  ]);
})();

const refreshFuncs: (() => Promise<void>)[] = [];

refreshFuncs.push(refreshStatus);
export async function refreshStatus() {
  const newState = await ffi.getStatus();
  if (newState !== state.status) {
    state.status = newState;
  }
}

refreshFuncs.push(refreshPath);
export async function refreshPath() {
  const newPath = await ffi.getRepoPath();
  if (newPath !== state.path) {
    state.path = newPath;
  }
}

export async function refreshAll() {
  for (const refreshFunc of refreshFuncs) {
    await refreshFunc();
  }
}

export async function openRepo(path: string) {
  try {
    await pollUntilComplete(ffi.openRepo(path), refreshStatus);
    await refreshStatus();
    state.path = await ffi.getRepoPath();
  } catch (e) {
    console.error(e);
    state.path = null;
  }
}

export async function promptOpenRepo() {
  let path = await open({ directory: true, multiple: false });
  if (Array.isArray(path)) throw "cannot open multiple directories";

  if (path !== null) {
    await openRepo(path);
  }
}

export async function closeRepo() {
  await ffi.closeRepo();
  state.path = null;
}
