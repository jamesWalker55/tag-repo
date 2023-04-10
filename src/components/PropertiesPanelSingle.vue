<script lang="ts" setup>
import ItemIcon from "@/components/ItemIcon.vue";
import { determineFileType, FileType, getItem, Item } from "@/lib/api";
import { computed, reactive, Ref, ref, watch } from "vue";
import { basename, extname } from "@tauri-apps/api/path";

const props = defineProps<{ id: number }>();

const itemData: Ref<Item | null> = ref(null);

async function fetchItemData(id: number) {
  itemData.value = await getItem(id);
}

// fetch data asynchronously
fetchItemData(props.id).then();

watch(
  () => props.id,
  (newId: number) => {
    fetchItemData(newId);
  }
);

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
</script>

<template>
  <div class="flex flex-1 flex-row bg-red-500">
    <ItemIcon class="w-[48px] h-[48px]" :filetype="extraData.fileType" />
  </div>
</template>

<style scoped></style>
