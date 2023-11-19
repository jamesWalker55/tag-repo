import { defineStore } from "pinia";

export enum FixedComponents {
  itemList = "itemList",
}

export enum PanelComponent {
  FolderTree = "FolderTree",
  ItemProperties = "ItemProperties",
}

export enum ItemListColumnType {
  Path = "Path",
  Name = "Name",
  Tags = "Tags",
  Extension = "Extension",
}

export interface ListViewColumn {
  // what kind of column this is
  type: ItemListColumnType;
  // width of the column in pixels
  width: number;
}

/**
 * Persistent state between sessions
 */
export interface WindowConfig {
  lastOpenedPath: string | null;
  dimensions: null | {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  audioPreview: {
    enabled: boolean;
    volume: number;
  };
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
  components: {
    itemList: {
      columns: ListViewColumn[];
    };
    folderTree: {
      recursive: boolean;
    };
  };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const INITIAL_CONFIG = (window as any).configPlugin as WindowConfig;

export const useConfigStore = defineStore("config", {
  state: (): WindowConfig => INITIAL_CONFIG,
  actions: {},
});
