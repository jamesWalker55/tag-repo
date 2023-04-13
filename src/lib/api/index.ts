import { watch } from "vue";
import { ManagerStatus, insertTags, removeTags } from "@/lib/ffi";
import { Event, listen } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";
import { refreshAll, state } from "./state";
import { closeRepo, openRepo, promptOpenRepo } from "./repo";
import { setQuery } from "./query";
import { type ListViewColumn } from "@/lib/api/view-columns";
import {
  type ItemDetails,
  queryItemIds,
  clearItemCache,
  setCachedItem,
  requestItemToBeFetched,
} from "./items";
import { selection } from "./selection";
import * as actions from "./actions";

export {
  revealFile,
  launchFile,
  determineFileType,
  FileType,
  getFolders,
  type Folder,
  supportsAudioPlayback,
  previewAudio,
  stopAudio,
  getAudioVolume,
  setAudioVolume,
} from "@/lib/ffi";
export {
  type ItemDetails,
  type ListViewColumn,
  ManagerStatus,
  openRepo,
  promptOpenRepo,
  closeRepo,
  setQuery,
  state,
  refreshAll,
  selection,
  insertTags,
  removeTags,
  requestItemToBeFetched,
  actions,
};

// listen to change events from the backend
(async () => {
  await Promise.all([
    listen("item-added", async (evt: Event<ItemDetails>) => {
      console.log("item-added", evt);
      // update item list without discarding cache
      state.itemIds = await queryItemIds(state.query);
    }),
    listen("item-removed", async (evt: Event<ItemDetails>) => {
      console.log("item-removed", evt);
      // remove the item from the item list, if it exists
      const index = state.itemIds.indexOf(evt.payload.item.id);
      if (index !== -1) {
        state.itemIds.splice(index, 1);
      }
      // remove from selection if it's in it
      try {
        selection.remove(evt.id);
      } catch (e) {
        // nothing
      }
    }),
    listen("item-renamed", async (evt: Event<ItemDetails>) => {
      console.log("item-renamed", evt);
      // put item into cache, replacing if it already exists
      setCachedItem(evt.payload.item.id, evt.payload);
    }),
    listen("status-changed", (evt: Event<ManagerStatus | null>) => {
      console.log("Status changed to:", evt.payload);
      state.status = evt.payload;
    }),
    listen("repo-path-changed", (evt: Event<string>) => {
      state.path = evt.payload;
      selection.clear();
    }),
    listen("repo-resynced", async (evt: Event<string>) => {
      const newItems = await queryItemIds(state.query);
      clearItemCache();
      state.itemIds = newItems;
      // clear the selection for now
      // TODO: You should make the resync code emit an event containing removed paths, so
      //  you can remove them from the selection
      selection.clear();
    }),
    listen("item-tags-added", async (evt: Event<ItemDetails>) => {
      console.log("item-tags-added", evt);
      setCachedItem(evt.payload.item.id, evt.payload);
    }),
    listen("batch-item-tags-added", async (evt: Event<ItemDetails[]>) => {
      console.log("batch-item-tags-added", evt);
      for (const itemDetail of evt.payload) {
        setCachedItem(itemDetail.item.id, itemDetail);
      }
    }),
    listen("item-tags-removed", async (evt: Event<ItemDetails>) => {
      console.log("item-tags-removed", evt);
      setCachedItem(evt.payload.item.id, evt.payload);
    }),
    listen("batch-item-tags-removed", async (evt: Event<ItemDetails[]>) => {
      console.log("batch-item-tags-removed", evt);
      for (const itemDetail of evt.payload) {
        setCachedItem(itemDetail.item.id, itemDetail);
      }
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
    // clear selection, we're in a new repo now
    selection.clear();
    // execute several async functions at once
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
    // clear selection, we're moving to a new item list
    selection.clear();
    // clear item cache in case the item has changed upstream
    clearItemCache();
    // actually change the item list
    state.itemIds = newItems;
  }
);
