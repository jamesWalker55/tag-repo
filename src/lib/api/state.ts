import * as ffi from "@/lib/ffi";
import { ItemDetails, ManagerStatus, supportsAudioPlayback } from "@/lib/ffi";
import { Selection } from "./selection";
import { defineStore } from "pinia";

/**
 * Temporary state that is lost between sessions
 */
export interface WindowState {
  repo: null | {
    path: string;
    status: ManagerStatus;
  };
  // the currently-displayed query
  query: string;
  // a boolean that updates whenever you execute a search, indicating any query errors
  queryError: boolean;
  // the currently-displayed item list
  itemIds: number[];
  // the item cache, this will be changed regularly
  itemCache: Record<number, ItemDetails | undefined>;

  // the selection in the list view
  itemIdSelection: Selection | null;
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const useStateStore = defineStore("state", {
  state: (): WindowState => ({
    repo: null,
    query: "",
    queryError: false,
    itemIds: [],
    itemCache: {},
    itemIdSelection: null,
  }),
  actions: {
    async refreshAll() {
      await this.refreshStatus();
      await this.refreshPath();
      await this.refreshAudioVolume();
    },
    async refreshStatus() {
      const newState = await ffi.getStatus();
      if (newState !== this.status) {
        this.status = newState;
      }
    },
    async refreshPath() {
      const newPath = await ffi.getRepoPath();
      if (newPath !== this.path) {
        this.path = newPath;
      }
    },
    async refreshAudioVolume() {
      const volume = await ffi.getAudioVolume();
      if (volume !== this.audioVolume) {
        this.audioVolume = volume;
      }
    },
  },
});

// export type PanelSizeKey = keyof WindowState["panelSizes"];

// check if audio device is supported, then enable audio preview now
supportsAudioPlayback().then((supported) => {
  if (supported) {
    state.audioPreview = true;
  }
});
