import { watch } from "vue";
import { ManagerStatus } from "@/lib/ffi";
import { Event, listen } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";
import { refreshAll, state } from "./state";
import { closeRepo, openRepo, promptOpenRepo } from "./repo";
import { setQuery } from "./query";
import { type ListViewColumn } from "@/lib/api/view-columns";
import {
  clearItemCache,
  getCachedItem,
  getItem,
  type Item,
  queryItemIds,
} from "./items";
import { selection } from "./selection";

export { revealFile, openFile, determineFileType, FileType } from "@/lib/ffi";
export {
  type Item,
  type ListViewColumn,
  ManagerStatus,
  openRepo,
  promptOpenRepo,
  closeRepo,
  setQuery,
  state,
  getItem,
  refreshAll,
  selection,
};

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

      const cachedItem = getCachedItem(evt.payload.id);
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
updateWindowTitle(state.path).then();

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
