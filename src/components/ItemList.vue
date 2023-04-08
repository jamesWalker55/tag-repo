<script lang="ts" setup>
import { RecycleScroller } from "vue-virtual-scroller";
import ItemRow from "./ItemRow.vue";
import { state } from "@/lib/api";
import { parseRemSize } from "@/lib/utils";
import tailwind, { getSpacingSize } from "@/lib/tailwindcss";

export interface Column {
  // what kind of column this is
  type: "path" | "name" | "tags";
  // width of the column in pixels
  width: number;
}

defineProps<{ columns: Column[] }>();

// set item height to Tailwind's 'h-8'
// keep this in sync with ItemRow's height
const itemSize = getSpacingSize("6");
</script>

<template>
  <RecycleScroller
    class="h-full min-h-full"
    listClass="hide-scrollbar !overflow-x-auto table"
    itemClass="flex !h-max !w-max"
    :items="state.itemIds"
    :item-size="itemSize"
    v-slot="{ item }"
    :key="state.path"
  >
    <ItemRow :id="item" :columns="columns" />
  </RecycleScroller>
</template>
