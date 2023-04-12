import * as ffi from "@/lib/ffi";
import { Item, ItemDetails } from "@/lib/ffi";
import { state } from "@/lib/api/state";

export type { Item, ItemDetails };

export async function queryItemIds(query: string) {
  console.log("querying with this:", query);
  return await ffi.queryItemIds(query);
}

export function clearItemCache() {
  state.itemCache = {};
}

export function setCachedItem(id: number, item: ItemDetails) {
  state.itemCache[id] = item;
}

export async function requestItemToBeFetched(id: number) {
  console.log("requesting item:", id);
  if (state.itemCache[id] === undefined) {
    state.itemCache[id] = await ffi.getItemDetails(id);
  }
}
