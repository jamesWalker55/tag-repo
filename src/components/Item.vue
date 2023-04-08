<script lang="ts" setup>
import { getItem, Item } from "@/lib/api";
import { Ref, ref, watch } from "vue";

const props = defineProps<{ id: number }>();
const itemData: Ref<Item | null> = ref(null);

async function fetchItemData(id: number) {
  console.log("Refreshing item:", id);
  itemData.value = await getItem(id);
}

fetchItemData(props.id);

watch(
  () => props.id,
  async (newId) => await fetchItemData(newId)
);
</script>

<template>
  <div v-if="itemData !== null" class="truncate">
    {{ itemData.id }}: {{ itemData.path }}
  </div>
  <div v-else>Loading...</div>
</template>

<style scoped></style>
