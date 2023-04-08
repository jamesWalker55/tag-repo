<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { refreshAll, state, openRepo } from "@/lib/api";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { determineFileType } from "@/lib/ffi";

refreshAll();

// path.basename("D:\\vm\\qmul-files\\yfp\\testrepo\\b\\b\\").then(console.log);

determineFileType("hello.wav").then(console.log);

// // Force the scroller to re-mount and update.
// //
// // This is used when we change repos, the `itemIds` list will change but some IDs
// // may remain in the same place by coincidence. When this happens the scroller reuses
// // the path names from the old repo since they happen to have the same ID.
// const scrollerRefreshBool = ref(false);
// watch(
//   () => state.itemIds,
//   () => (scrollerRefreshBool.value = !scrollerRefreshBool.value)
// );
</script>

<template>
  <div
    id="container"
    class="relative grid h-screen max-h-screen select-none grid-rows-app border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <main class="relative grid grid-rows-[max-content_minmax(0,_1fr)]">
      <QueryBar />
      <ItemList
        :columns="[
          { type: 'name', width: 300 },
          { type: 'path', width: 500 },
          { type: 'tags', width: 300 },
        ]"
      />
    </main>
    <StatusBar />
  </div>
</template>
