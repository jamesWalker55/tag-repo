import { reactive, watch } from "vue";
import { pollUntilComplete } from "@/lib/utils";
import type { Item } from "@/lib/ffi";
import * as ffi from "@/lib/ffi";
import { open } from "@tauri-apps/api/dialog";
import { Event, listen } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";

export type { Item } from "@/lib/ffi";
export { revealFile, openFile } from "@/lib/ffi";

interface AppState {
  path: string | null;
  status: string | null;
  query: string;
  itemIds: number[];
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: AppState = reactive({
  path: null,
  status: null,
  query: "",
  itemIds: [],
});

const itemCache: Map<number, Item> = new Map();

// listen to change events from the backend
(async () => {
  await Promise.all([
    listen("item-added", async (evt: Event<string>) => {
      console.log("item-added", evt);
      await queryItemIds(state.query);
    }),
    listen("item-removed", async (evt: Event<string>) => {
      console.log("item-removed", evt);
      await queryItemIds(state.query);
    }),
    listen("item-renamed", async (evt: Event<[string, string]>) => {
      console.log("item-renamed", evt);
      await queryItemIds(state.query);
    }),
    listen("status-changed", (evt: Event<string>) => {
      state.status = evt.payload;
    }),
    listen("repo-path-changed", (evt: Event<string>) => {
      state.path = evt.payload;
    }),
    listen("repo-resynced", async (evt: Event<string>) => {
      await queryItemIds(state.query);
    }),
    listen("tauri://file-drop", (event) => {
      console.log(event);
    }),
    listen("tauri://file-drop-cancelled", (event) => {
      console.log(event);
    }),
    listen("tauri://file-drop-hover", (event) => {
      console.log(event);
    }),
  ]);
})();

// update app title when the path changes
async function updateWindowTitle(path: string | null) {
  if (path === null) {
    await appWindow.setTitle("tagrepo");
  } else {
    await appWindow.setTitle(`${path} - tagrepo`);
  }
}

// update the window title now
updateWindowTitle(state.path);

// watchers
// when the path changes...
watch(
  () => state.path,
  async (newPath) => {
    await Promise.all([
      // update the window title
      updateWindowTitle(newPath),
      // re-query for new items
      (async () => {
        if (newPath === null) {
          state.itemIds = [];
        } else {
          await queryItemIds(state.query);
        }
      })(),
    ]);
  }
);
// when the query changes...
watch(
  () => state.query,
  async (newQuery) => {
    // re-query for new items
    await queryItemIds(newQuery);
  }
);

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
  await ffi.openRepo(path);
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

export function setQuery(query: string) {
  console.log("Set query to:", query);
  state.query = query;
}

async function queryItemIds(query: string) {
  console.log("querying with this:", query);
  const itemIds = await ffi.queryItemIds(query);
  clearItemCache();
  state.itemIds = itemIds;
}

export function clearItemCache() {
  itemCache.clear();
}

export async function getItem(
  id: number,
  cached: boolean = true
): Promise<Item> {
  if (cached) {
    let item = itemCache.get(id);
    if (item !== undefined) return item;
  }
  const item = await ffi.getItem(id);
  itemCache.set(id, item);
  return item;
}
