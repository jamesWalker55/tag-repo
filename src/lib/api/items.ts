import * as ffi from "@/lib/ffi";
import {Item, ItemDetails} from '@/lib/ffi';
import { ref, Ref } from "vue";

export type { Item, ItemDetails };

const itemCache: Ref<Map<number, ItemDetails>> = ref(new Map());

function itemIdsEqual(arr1: number[], arr2: number[]) {
  if (arr1.length !== arr2.length) return false;

  for (let i = 0; i < arr1.length; i++) {
    if (arr1[i] !== arr2[i]) {
      return false;
    }
  }
  return true;
}

export async function queryItemIds(query: string) {
  console.log("querying with this:", query);
  return await ffi.queryItemIds(query);
}

export function clearItemCache() {
  itemCache.value.clear();
}

export function getCachedItem(id: number): ItemDetails | null {
  return itemCache.value.get(id) || null;
}

export function setCachedItem(id: number, item: ItemDetails) {
  itemCache.value.set(id, item);
}

export async function getItemDetails(
  id: number,
  cached: boolean = true
): Promise<ItemDetails> {
  if (cached) {
    let item = itemCache.value.get(id);
    if (item !== undefined) return item;
  }
  const item = await ffi.getItemDetails(id);
  itemCache.value.set(id, item);
  return item;
}
