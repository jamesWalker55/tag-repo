<script lang="ts" setup>
import {
  determineFileType,
  FileType,
  getItem,
  Item,
  selection,
  state,
} from "@/lib/api";
import { computed, reactive, Ref, ref, watch } from "vue";
import ItemIcon from "@/components/ItemIcon.vue";
import { basename, extname } from "@tauri-apps/api/path";

interface Props {
  // the item id of this row
  id: number;
  // the index of the item in state.itemIds
  listIndex: number;
}
const props = defineProps<Props>();
const emit = defineEmits<{
  // select a single item with normal mouse click
  (e: "selection-set", id: number): void;
  // add to selection with ctrl + mouse click
  (e: "selection-add", id: number): void;
  // remove from selection with ctrl + mouse click
  (e: "selection-remove", id: number): void;
  // extend selection with shift + mouse click
  (e: "selection-extend", id: number): void;
}>();

const itemData: Ref<Item | null> = ref(null);
const isSelected = computed(() => selection.contains(props.listIndex));

interface ExtraData {
  fileType: FileType;
  filename: string;
  extension: string;
}

const extraData: ExtraData = reactive({
  fileType: FileType.UNKNOWN,
  filename: "",
  extension: "",
});

async function fetchItemData(id: number) {
  itemData.value = await getItem(id);
}

// fetch data asynchronously
fetchItemData(props.id).then();

// this watch has 2 causes:
// 1. the initial data fetch
//    - the #fetchItemData() call above will asynchronously set itemData.path
// 2. watch when the item cache changes
//    - this can be caused by the watcher sending "rename" events
watch(
  () => itemData.value?.path,
  (newPath) => {
    if (newPath !== undefined) {
      determineFileType(newPath)
        .then((x) => (extraData.fileType = x))
        .catch(console.error);
      basename(newPath)
        .then((x) => (extraData.filename = x))
        .catch(console.error);
      extname(newPath)
        .then((x) => (extraData.extension = x))
        .catch(() => {
          // path has no extension
          extraData.extension = "";
        });
    } else {
      extraData.fileType = FileType.UNKNOWN;
      extraData.filename = "";
      extraData.extension = "";
    }
  }
);

console.log("selection:", selection);
watch(
  () => state.itemIdSelection,
  (sel) => console.log(sel)
);
const log = console.log;
</script>

<template>
  <div
    v-if="itemData !== null"
    class="item flex h-6 w-full min-w-max items-center hover:bg-slate-50"
    :class="
      !isSelected
        ? 'hover:bg-slate-50 hover:outline hover:outline-1 hover:outline-neutral-200'
        : 'bg-sky-200 outline outline-1 outline-sky-300 hover:bg-sky-200 hover:outline-sky-400'
    "
    @mousedown="
      (e: MouseEvent) => {
        if (state.path === null) throw 'repo path is null?!';
        if (itemData === null) throw 'item data is null?!';

        if (e.shiftKey && e.ctrlKey) {
          selection.addTo(listIndex);
        } else if (e.shiftKey) {
          selection.extendTo(listIndex);
        } else if (e.ctrlKey) {
          if (isSelected) {
            selection.remove(listIndex);
          } else {
            selection.add(listIndex);
          }
        } else {
          selection.isolate(listIndex);
        }
        // await clipboard.writeText(await path.join(state.path, itemData.path));
        // await revealFile(await join(state.path, itemData.path));
      }
    "
  >
    <!-- v-if has higher priority than v-for, see https://vuejs.org/guide/essentials/list.html#v-for-with-v-if -->
    <template v-for="col in state.listViewColumns">
      <div
        v-if="col.type === 'name'"
        class="flex flex-nowrap gap-1 px-1"
        :style="{ width: `${col.width}px` }"
      >
        <ItemIcon
          :filetype="extraData.fileType"
          class="h-[16px] w-[16px] flex-none text-neutral-600"
        />
        <span class="flex-1 overflow-clip whitespace-nowrap">
          {{ extraData.filename }}
        </span>
      </div>
      <div
        v-else-if="col.type === 'path'"
        class="flex truncate px-1 text-neutral-700"
        :style="{ width: `${col.width}px` }"
      >
        {{ itemData.path }}
      </div>
      <div
        v-else-if="col.type === 'tags'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        <span v-if="itemData.tags">{{ itemData.tags }}</span>
        <span v-else class="italic" :class="!isSelected ? 'text-neutral-300' : 'text-neutral-400'">(no tags)</span>
      </div>
      <div
        v-else-if="col.type === 'extension'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        {{ extraData.extension }}
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
