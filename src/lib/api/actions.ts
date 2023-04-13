import { launchFile, revealFile } from "@/lib/ffi";
import { state } from "@/lib/api/state";
import { selection } from "@/lib/api/selection";
import { requestItemToBeFetched } from "@/lib/api/items";
import path from "path-browserify";
import { clipboard } from "@tauri-apps/api";
import { normalize } from "@tauri-apps/api/path";

/**
 * Attempt to get a list of all items. If they aren't loaded yet, request them to be loaded and return null.
 */
async function getSelectedItemFullPaths(): Promise<string[] | null> {
  const repoPath = state.path;
  if (repoPath === null) return null;

  // get items and check if all items are in cache
  let itemPaths = [];
  let notAllItemsLoaded = false;
  for (const index of selection.selected.value) {
    const itemId = state.itemIds[index];
    const item = state.itemCache[itemId];
    if (item === undefined) {
      requestItemToBeFetched(itemId).then();
      notAllItemsLoaded = true;
      continue;
    }
    // this path contains both forward and backward slashes on windows
    const itemPath = path.join(repoPath, item.item.path);
    itemPaths.push(itemPath);
  }
  if (notAllItemsLoaded) return null;

  // normalise paths (needed on windows, not sure on other systems)
  const normalizedPaths = await Promise.all(itemPaths.map((p) => normalize(p)));

  return normalizedPaths;
}

export async function launchSelectedItems() {
  let itemPaths = await getSelectedItemFullPaths();
  if (itemPaths === null) return;

  for (const itemPath of itemPaths) {
    await launchFile(itemPath);
  }
}

export async function revealSelectedItems() {
  let itemPaths = await getSelectedItemFullPaths();
  if (itemPaths === null) return;

  for (const itemPath of itemPaths) {
    await revealFile(itemPath);
  }
}

export async function copySelectedItemPaths() {
  let itemPaths = await getSelectedItemFullPaths();
  if (itemPaths === null) return;

  await clipboard.writeText(itemPaths.join("\n"));
}

export function shuffleList() {
  const itemIds = state.itemIds;
  for (var i = itemIds.length - 1; i > 0; i--) {
    var j = Math.floor(Math.random() * (i + 1));
    var temp = itemIds[i];
    itemIds[i] = itemIds[j];
    itemIds[j] = temp;
  }
}
