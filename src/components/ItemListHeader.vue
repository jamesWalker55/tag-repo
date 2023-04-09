<script lang="ts" setup>
import { RecycleScroller } from "vue-virtual-scroller";
import { state } from "@/lib/api";
import { parseRemSize } from "@/lib/utils";
import tailwind, { getSpacingSize } from "@/lib/tailwindcss";
</script>

<template>
  <div
    class="sticky top-0 flex h-6 w-full min-w-max border-b border-neutral-300 bg-neutral-100"
  >
    <template v-for="col in state.listViewColumns">
      <div
        v-if="col.type === 'path'"
        class="flex flex-none items-center border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px` }"
        @click="col.width += 5"
      >
        Path
      </div>
      <div
        v-else-if="col.type === 'tags'"
        class="flex flex-none items-center border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px` }"
        @click="col.width += 5"
      >
        Tags
      </div>
      <div
        v-else-if="col.type === 'extension'"
        class="flex flex-none items-center border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px` }"
        @click="col.width += 5"
      >
        Extension
      </div>
      <div
        v-else-if="col.type === 'name'"
        class="flex flex-none items-center border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px` }"
        @click="col.width += 5"
      >
        Name
      </div>
      <div
        v-else
        class="flex flex-none items-center border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px` }"
        @click="col.width += 5"
      >
        El
      </div>
      <div class="w-3 bg-red-500 opacity-50 cursor-col-resize"></div>
    </template>
  </div>
</template>
