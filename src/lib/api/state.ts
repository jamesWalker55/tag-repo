import { reactive } from "vue";
import * as ffi from "@/lib/ffi";
import {ItemDetails, ManagerStatus} from '@/lib/ffi';
import { Selection } from "./selection";
import { ListViewColumn } from "./view-columns";

export interface AppState {
  // the repo path, will be null if no repo loaded
  path: string | null;
  // the status of the repo, will be null if no repo loaded
  status: ManagerStatus | null;
  // the currently-displayed query
  query: string;
  // the currently-displayed item list
  itemIds: number[];
  // the item cache, this will be changed regularly
  itemCache: Record<number, ItemDetails | undefined>;
  // the headers/columns displayed in the list view
  listViewColumns: ListViewColumn[];
  // the selection in the list view
  itemIdSelection: Selection | null;
  // app panels
  panelSizes: {
    bottomPanel: number;
    leftPanel: number;
    rightPanel: number;
  };
}

// The app state. DO NOT MODIFY FROM CHILD COMPONENTS.
// You should only modify this using functions in this module.
export const state: AppState = reactive({
  path: null,
  status: null,
  query: "",
  itemIds: [],
  itemCache: {},
  listViewColumns: [
    { type: "name", width: 300 },
    { type: "tags", width: 160 },
    { type: "extension", width: 60 },
    { type: "path", width: 500 },
  ],
  itemIdSelection: null,
  // size of various panels
  panelSizes: {
    bottomPanel: 160,
    leftPanel: 200,
    rightPanel: 250,
  },
});

export type PanelSizeKey = keyof AppState["panelSizes"];

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

// fetch data right now
refreshAll().then();
