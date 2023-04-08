<script lang="ts" setup>
import { RecycleScroller } from "vue-virtual-scroller";
import ItemRow, { Column } from "./ItemRow.vue";
import { state } from "@/lib/api";
import { parseRemSize } from "@/lib/utils";
import tailwind, { getSpacingSize } from "@/lib/tailwindcss";

defineProps<{ columns: Column[] }>();
</script>

<template>
  <div class="flex h-6 border-b overflow-x-clip sticky top-0">
    <template v-for="col in columns">
      <div
        v-if="col.type === 'path'"
        class="flex flex-none items-center border-r px-2"
        :style="{ width: `${col.width}px` }"
      >
        Path
      </div>
      <div
        v-else-if="col.type === 'tags'"
        class="flex flex-none items-center border-r px-2"
        :style="{ width: `${col.width}px` }"
      >
        Tags
      </div>
      <div
        v-else-if="col.type === 'extension'"
        class="flex flex-none items-center border-r px-2"
        :style="{ width: `${col.width}px` }"
      >
        Extension
      </div>
      <div
        v-else-if="col.type === 'name'"
        class="flex flex-none items-center border-r px-2"
        :style="{ width: `${col.width}px` }"
      >
        Name
      </div>
      <div
        v-else
        class="flex flex-none items-center border-r px-2"
        :style="{ width: `${col.width}px` }"
      >
        El
      </div>
    </template>
  </div>
</template>
