import * as ffi from "@/lib/ffi";
import { Item } from "@/lib/ffi";
import { ref, Ref } from "vue";

export { type Item };

const itemCache: Ref<Map<number, Item>> = ref(new Map());

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

export function getCachedItem(id: number): Item | undefined {
  return itemCache.value.get(id);
}

export async function getItem(
  id: number,
  cached: boolean = true
): Promise<Item> {
  if (cached) {
    let item = itemCache.value.get(id);
    if (item !== undefined) return item;
  }
  const item = await ffi.getItem(id);
  itemCache.value.set(id, item);
  return item;
}
