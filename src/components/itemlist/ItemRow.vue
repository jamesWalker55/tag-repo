<script lang="ts" setup>
import {
  determineFileType,
  FileType,
  ItemDetails,
  requestItemToBeFetched,
  selection,
  state,
} from "@/lib/api";
import { computed, reactive, Ref, ref, watch } from "vue";
import ItemIcon from "@/components/itemlist/ItemIcon.vue";
import path from "path-browserify";
import { tagsToString } from "@/lib/utils";

interface Props {
  // the item id of this row
  id: number;
  // the index of the item in state.itemIds
  listIndex: number;
}
const props = defineProps<Props>();

requestItemToBeFetched(props.id);

watch(
  () => state.itemCache,
  (newItemData) => {
    if (newItemData[props.id] === undefined) {
      // this means the item cache was cleared, we need to reload it
      requestItemToBeFetched(props.id);
    }
  }
);

const isSelected = computed(() => selection.contains(props.listIndex));

function onItemMouseDown(e: MouseEvent) {
  // only allow left mouse click
  if ((e.buttons & 1) !== 1) {
    return;
  }

  if (e.shiftKey && e.ctrlKey) {
    selection.addTo(props.listIndex);
  } else if (e.shiftKey) {
    selection.extendTo(props.listIndex);
  } else if (e.ctrlKey) {
    if (isSelected.value) {
      selection.remove(props.listIndex);
    } else {
      selection.add(props.listIndex);
    }
  } else {
    selection.isolate(props.listIndex);
  }
  // await clipboard.writeText(await path.join(state.path, itemData.path));
  // await revealFile(await join(state.path, itemData.path));
}
</script>

<template>
  <div
    v-if="state.itemCache[id] !== undefined"
    class="item flex h-6 w-full min-w-max items-center"
    :class="
      !isSelected
        ? 'hover:bg-slate-50 hover:outline hover:outline-1 hover:outline-neutral-200'
        : 'bg-sky-200 outline outline-1 outline-sky-300 hover:bg-sky-200 hover:outline-sky-400'
    "
    @click.stop
    @mousedown="onItemMouseDown"
  >
    <!-- v-if has higher priority than v-for, see https://vuejs.org/guide/essentials/list.html#v-for-with-v-if -->
    <template v-for="col in state.listViewColumns">
      <div
        v-if="col.type === 'name'"
        class="flex flex-nowrap gap-1 px-1"
        :style="{ width: `${col.width}px` }"
      >
        <ItemIcon
          :filetype="state.itemCache[id]!.filetype"
          class="h-16px w-16px flex-none text-neutral-600"
        />
        <span class="flex-1 overflow-clip whitespace-nowrap">
          {{ path.basename(state.itemCache[id]!.item.path) }}
        </span>
      </div>
      <div
        v-else-if="col.type === 'path'"
        class="flex truncate px-1 text-neutral-700"
        :style="{ width: `${col.width}px` }"
      >
        {{ state.itemCache[id]!.item.path }}
      </div>
      <div
        v-else-if="col.type === 'tags'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        <span v-if="state.itemCache[id]!.item.tags.length > 0">
          {{ tagsToString(state.itemCache[id]!.item.tags) }}
        </span>
        <span
          v-else
          class="italic"
          :class="!isSelected ? 'text-neutral-300' : 'text-neutral-400'"
        >
          (no tags)
        </span>
      </div>
      <div
        v-else-if="col.type === 'extension'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        {{ path.extname(state.itemCache[id]!.item.path) }}
      </div>
      <div
        v-else
        class="flex truncate px-1 italic text-red-500"
        :style="{ width: `${col.width}px` }"
      >
        Not implemented, please notify the developer!
      </div>
    </template>
  </div>
  <div v-else>Loading...</div>
</template>

<style scoped>
.hover > .item {
  @apply bg-blue-50;
}
</style>
