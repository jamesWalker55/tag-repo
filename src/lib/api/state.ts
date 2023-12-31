import * as ffi from "@/lib/ffi";
import { ItemDetails, ManagerStatus, supportsAudioPlayback } from "@/lib/ffi";
import { reactive } from "vue";
import { Selection } from "./selection";
import { ListViewColumn } from "./view-columns";

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
  // list of all tags
  tags: ffi.Tag[];
  // the headers/columns displayed in the list view
  listViewColumns: ListViewColumn[];
  // whether the folder tree panel should be recursive
  recursiveTree: boolean;
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

const initialConfig = (window as any).configPlugin;

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: WindowState = reactive({
  path: initialConfig.path,
  status: null,
  query: "",
  queryIsInvalid: false,
  itemIds: [],
  itemCache: {},
  tags: [],
  listViewColumns: initialConfig.components.itemList.columns,
  recursiveTree: initialConfig.components.folderTree.recursive,
  itemIdSelection: null,
  audioPreview: initialConfig.audioPreview.enabled,
  audioVolume: initialConfig.audioPreview.volume,
  // size of various panels
  panelSizes: {
    bottomPanel: initialConfig.layout.bottom.size,
    leftPanel: initialConfig.layout.left.size,
    rightPanel: initialConfig.layout.right.size,
  },
  panelVisibility: {
    bottomPanel: initialConfig.layout.bottom.component !== null,
    leftPanel: initialConfig.layout.left.component !== null,
    rightPanel: initialConfig.layout.right.component !== null,
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

refreshFuncs.push(refreshTags);
export async function refreshTags() {
  state.tags = await ffi.tags();
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
