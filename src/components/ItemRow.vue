<script lang="ts" setup>
import { determineFileType, FileType, getItem, Item, state } from "@/lib/api";
import { reactive, Ref, ref, watch } from "vue";
import ItemIcon from "@/components/ItemIcon.vue";
import { basename, extname } from "@tauri-apps/api/path";

export interface Column {
  // what kind of column this is
  type: "path" | "name" | "tags" | "extension";
  // width of the column in pixels
  width: number;
}

const props = defineProps<{ id: number; columns: Column[] }>();
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
  const data = await getItem(id);
  itemData.value = data;
  determineFileType(data.path)
    .then((x) => (extraData.fileType = x))
    .catch(console.error);
  basename(data.path)
    .then((x) => (extraData.filename = x))
    .catch(console.error);
  extname(data.path)
    .then((x) => (extraData.extension = x))
    .catch(() => {
      // path has no extension
      extraData.extension = "";
    });
}

fetchItemData(props.id);

watch(
  () => props.id,
  async (newId) => {
    await fetchItemData(newId);
  }
);
</script>

<template>
  <div
    v-if="itemData !== null"
    class="item flex h-6 items-center px-1"
    @click="
      async () => {
        if (state.path === null) throw 'repo path is null?!';
        if (itemData === null) throw 'item data is null?!';

        // await clipboard.writeText(await path.join(state.path, itemData.path));
        // await revealFile(await path.join(state.path, itemData.path));
      }
    "
  >
    <!-- v-if has higher priority than v-for, see https://vuejs.org/guide/essentials/list.html#v-for-with-v-if -->
    <template v-for="col in columns">
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
        <span v-else class="italic text-neutral-300">(no tags)</span>
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
