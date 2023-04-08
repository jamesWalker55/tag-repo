<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { refreshAll, state, openRepo } from "@/lib/api";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { determineFileType } from "@/lib/ffi";
import { Column } from "@/components/ItemRow.vue";

refreshAll();

// path.basename("D:\\vm\\qmul-files\\yfp\\testrepo\\b\\b\\").then(console.log);

determineFileType("hello.wav").then(console.log);

const columns: Column[] = [
  { type: "name", width: 300 },
  { type: "extension", width: 100 },
  { type: "path", width: 500 },
  { type: "tags", width: 300 },
];
</script>

<template>
  <div
    id="container"
    class="relative grid h-screen max-h-screen select-none grid-rows-app border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <main class="relative grid grid-rows-[max-content_minmax(0,_1fr)]">
      <QueryBar />
      <ItemList :columns="columns" />
    </main>
    <StatusBar />
  </div>
</template>
