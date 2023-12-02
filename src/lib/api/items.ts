import { state } from "@/lib/api/state";
import * as ffi from "@/lib/ffi";
import { Item, ItemDetails } from "@/lib/ffi";

export type { Item, ItemDetails };

export async function queryItemIds(query: string): Promise<number[]> {
  console.log("querying with this:", query);
  try {
    const itemIds = await ffi.queryItemIds(query);
    state.queryIsInvalid = false;
    return itemIds;
  } catch (e) {
    state.queryIsInvalid = true;
    throw e;
  }
}

export function clearItemCache() {
  state.itemCache = {};
}

export function setCachedItem(id: number, item: ItemDetails) {
  state.itemCache[id] = item;
}

export async function requestItemToBeFetched(id: number) {
  if (state.itemCache[id] === undefined) {
    state.itemCache[id] = await ffi.getItemDetails(id);
  }
}
