import { reactive } from "vue";
import * as ffi from "@/lib/ffi";
import { ItemDetails, ManagerStatus, supportsAudioPlayback } from "@/lib/ffi";
import { Selection } from "./selection";

enum PanelComponent {
  folderTree = "folderTree",
  itemProperties = "itemProperties",
}

export interface ListViewColumn {
  // what kind of column this is
  type: "path" | "name" | "tags" | "extension";
  // width of the column in pixels
  width: number;
}

/**
 * Persistent state between sessions
 */
export interface WindowConfig {
  // the path that is open in the previous session, try to re-open in this session
  lastOpenPath: string;
  audioPreview: {
    // whether the user wants to have audio preview
    // note this may be true even if the system doesn't support it
    enabled: boolean;
    // 1.0 is normal, 0.5 is half-volume
    volume: number;
  };
  // position of components, size of panels etc
  layout: {
    left: {
      visible: boolean;
      size: number;
      component: PanelComponent;
    };
    right: {
      visible: boolean;
      size: number;
      component: PanelComponent;
    };
    bottom: {
      visible: boolean;
      size: number;
      component: PanelComponent;
    };
  };
  // persistent settings for each component
  components: {
    itemList: {
      columns: ListViewColumn[];
    };
    [PanelComponent.folderTree]: {
      recursive: boolean;
    };
    // [PanelComponent.itemProperties]: {};
  };
}

/**
 * Temporary state that is lost between sessions
 */
export interface WindowState {
  // the repo path, will be null if no repo loaded
  path: string | null;
  // the status of the repo, will be null if no repo loaded
  status: ManagerStatus | null;
  // the currently-displayed query
  query: string;
  // a boolean that updates whenever you execute a search, indicating any query errors
  queryIsInvalid: boolean;
  // the currently-displayed item list
  itemIds: number[];
  // the item cache, this will be changed regularly
  itemCache: Record<number, ItemDetails | undefined>;
  // the headers/columns displayed in the list view
  listViewColumns: ListViewColumn[];
  // the selection in the list view
  itemIdSelection: Selection | null;
  // whether audio previewing is enabled
  audioPreview: boolean;
  // playback volume
  audioVolume: number;
  // app panels
  panelSizes: {
    bottomPanel: number;
    leftPanel: number;
    rightPanel: number;
  };
  panelVisibility: {
    bottomPanel: boolean;
    leftPanel: boolean;
    rightPanel: boolean;
  };
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: WindowState = reactive({
  path: null,
  status: null,
  query: "",
  queryIsInvalid: false,
  itemIds: [],
  itemCache: {},
  listViewColumns: [
    { type: "name", width: 300 },
    { type: "tags", width: 160 },
    { type: "extension", width: 60 },
    { type: "path", width: 500 },
  ],
  itemIdSelection: null,
  audioPreview: false,
  audioVolume: 0.5,
  // size of various panels
  panelSizes: {
    bottomPanel: 160,
    leftPanel: 200,
    rightPanel: 250,
  },
  panelVisibility: {
    bottomPanel: false,
    leftPanel: true,
    rightPanel: true,
  },
});

export type PanelSizeKey = keyof WindowState["panelSizes"];

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

refreshFuncs.push(refreshAudioVolume);
export async function refreshAudioVolume() {
  const volume = await ffi.getAudioVolume();
  if (volume !== state.audioVolume) {
    state.audioVolume = volume;
  }
}

export async function refreshAll() {
  for (const refreshFunc of refreshFuncs) {
    await refreshFunc();
  }
}

// fetch data right now
refreshAll().then();

// check if audio device is supported, then enable audio preview now
supportsAudioPlayback().then((supported) => {
  if (supported) {
    state.audioPreview = true;
  }
});
