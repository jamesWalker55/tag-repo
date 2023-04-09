import { reactive, Ref, ref, watch } from "vue";
import type { Item } from "@/lib/ffi";
import * as ffi from "@/lib/ffi";
import { ManagerStatus } from "@/lib/ffi";
import { open } from "@tauri-apps/api/dialog";
import { Event, listen } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";

export type { Item } from "@/lib/ffi";
export { revealFile, openFile } from "@/lib/ffi";
export { FileType } from "@/lib/ffi";
export { determineFileType } from "@/lib/ffi";
export { ManagerStatus } from "@/lib/ffi";

export interface ListViewColumn {
  // what kind of column this is
  type: "path" | "name" | "tags" | "extension";
  // width of the column in pixels
  width: number;
}

interface AppState {
  path: string | null;
  status: ManagerStatus | null;
  query: string;
  itemIds: number[];
  listViewColumns: ListViewColumn[];
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: AppState = reactive({
  path: null,
  status: null,
  query: "",
  itemIds: [],
  listViewColumns: [
    { type: "name", width: 300 },
    { type: "extension", width: 60 },
    { type: "path", width: 500 },
    { type: "tags", width: 200 },
  ],
});

const itemCache: Ref<Map<number, Item>> = ref(new Map());

function itemIdsEqual(arr1: number[], arr2: number[]) {
  if (arr1.length !== arr2.length) return false;

  for (let i = 0; i < arr1.length; i++) {
    if (arr1[i] !== arr2[i]) {
      return false;
    }
  }
  return true;
}

// listen to change events from the backend
(async () => {
  await Promise.all([
    listen("item-added", async (evt: Event<Item>) => {
      console.log("item-added", evt);
      // update item list without discarding cache
      state.itemIds = await queryItemIds(state.query);
    }),
    listen("item-removed", async (evt: Event<Item>) => {
      console.log("item-removed", evt);
      // remove the item from the item list, if it exists
      const index = state.itemIds.indexOf(evt.payload.id);
      if (index !== -1) {
        state.itemIds.splice(index, 1);
      }
    }),
    listen("item-renamed", async (evt: Event<Item>) => {
      console.log("item-renamed", evt);
      // edit the item cache to change the path

      const cachedItem = itemCache.value.get(evt.payload.id);
      if (cachedItem !== undefined) {
        cachedItem.path = evt.payload.path;
      }
    }),
    listen("status-changed", (evt: Event<ManagerStatus | null>) => {
      console.log("Status changed to:", evt.payload);
      state.status = evt.payload;
    }),
    listen("repo-path-changed", (evt: Event<string>) => {
      state.path = evt.payload;
    }),
    listen("repo-resynced", async (evt: Event<string>) => {
      const newItems = await queryItemIds(state.query);
      clearItemCache();
      state.itemIds = newItems;
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
          const newItems = await queryItemIds(state.query);
          clearItemCache();
          state.itemIds = newItems;
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
    const newItems = await queryItemIds(state.query);
    clearItemCache();
    state.itemIds = newItems;
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
  return await ffi.queryItemIds(query);
}

export function clearItemCache() {
  itemCache.value.clear();
}

export async function getItem(
  id: number,
  cached: boolean = true
): Promise<Item> {
  if (cached) {
    let item = itemCache.value.get(id);
    if (item !== undefined) return item;
  }
  const item = await ffi.getItem(id);
  itemCache.value.set(id, item);
  return item;
}
