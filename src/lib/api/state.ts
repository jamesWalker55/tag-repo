import { reactive } from "vue";
import * as ffi from "@/lib/ffi";
import { ItemDetails, ManagerStatus, supportsAudioPlayback } from "@/lib/ffi";
import { Selection } from "./selection";

enum FixedComponents {
  itemList = "itemList",
}

enum PanelComponent {
  folderTree = "folderTree",
  itemProperties = "itemProperties",
}

type Component = FixedComponents | PanelComponent;

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
  // the path that was open in the previous session, try to re-open in this session
  lastOpenPath: string | null;
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
      component: PanelComponent | null;
      size: number;
    };
    right: {
      component: PanelComponent | null;
      size: number;
    };
    bottom: {
      component: PanelComponent | null;
      size: number;
    };
  };
  // persistent settings for each component
  components: {
    [FixedComponents.itemList]: {
      columns: ListViewColumn[];
    };
    [PanelComponent.folderTree]: {
      recursive: boolean;
    };
  };
}

export const config: WindowConfig = {
  lastOpenPath: null,
  audioPreview: {
    enabled: true,
    volume: 0.5,
  },
  layout: {
    left: {
      component: PanelComponent.folderTree,
      size: 200,
    },
    right: {
      component: PanelComponent.itemProperties,
      size: 250,
    },
    bottom: {
      component: null,
      size: 160,
    },
  },
  components: {
    [FixedComponents.itemList]: {
      columns: [
        { type: "name", width: 300 },
        { type: "tags", width: 160 },
        { type: "extension", width: 60 },
        { type: "path", width: 500 },
      ],
    },
    [PanelComponent.folderTree]: {
      recursive: false,
    },
  },
};

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
export const state: WindowState = reactive({
  repo: null,
  query: "",
  queryError: false,
  itemIds: [],
  itemCache: {},
  itemIdSelection: null,
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
