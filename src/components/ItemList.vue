<script lang="ts" setup>
import { RecycleScroller } from "vue-virtual-scroller";
import ItemListHeader from "@/components/ItemListHeader.vue";
import ItemRow, { Column } from "./ItemRow.vue";
import { state } from "@/lib/api";
import { parseRemSize } from "@/lib/utils";
import tailwind, { getSpacingSize } from "@/lib/tailwindcss";

defineProps<{ columns: Column[] }>();

// set item height to Tailwind's 'h-8'
// keep this in sync with ItemRow's height
const itemSize = getSpacingSize("6");
</script>

<template>
  <RecycleScroller
    class="h-full min-h-full text-sm relative"
    listClass="hide-scrollbar !overflow-x-auto"
    itemClass="flex !h-max !w-max"
    :items="state.itemIds"
    :item-size="itemSize"
    :key="state.path"
  >
    <template #before>
      <ItemListHeader :columns="columns" />
    </template>
    <template v-slot="{ item }">
      <ItemRow :id="item" :columns="columns" />
    </template>
  </RecycleScroller>
</template>
