<script lang="ts" setup>
import { determineFileType, FileType, getItem, Item, state } from "@/lib/api";
import { reactive, Ref, ref, watch } from "vue";
import ItemIcon from "@/components/ItemIcon.vue";
import { basename } from "@tauri-apps/api/path";

interface Column {
  // what kind of column this is
  type: "path" | "name" | "tags";
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
}

const extraData: ExtraData = reactive({
  fileType: FileType.UNKNOWN,
  filename: "",
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
    class="item flex h-6 items-center"
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
        class="flex flex-nowrap px-1"
        :style="{ width: `${col.width}px` }"
      >
        <ItemIcon
          :filetype="extraData.fileType"
          class="h-[16px] w-[16px] flex-none text-neutral-600"
        />
        <span class="flex-1 overflow-clip whitespace-nowrap text-sm">
          {{ extraData.filename }}
        </span>
      </div>
      <div
        v-if="col.type === 'path'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        <span class="text-sm">{{ itemData.path }}</span>
      </div>
      <div
        v-if="col.type === 'tags'"
        class="flex truncate px-1"
        :style="{ width: `${col.width}px` }"
      >
        <span v-if="itemData.tags">
          {{ itemData.tags }}
        </span>
        <span v-else class="italic text-neutral-300">
          (no tags)
        </span>
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
