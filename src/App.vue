<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import { refreshAll, state } from "@/lib/api";

import { appWindow } from "@tauri-apps/api/window";
import { watch } from "vue";

async function updateWindowTitle(path: string | null) {
  if (path === null) {
    await appWindow.setTitle("tagrepo");
  } else {
    await appWindow.setTitle(`${path} - tagrepo`);
  }
}

(async () => {
  watch(() => state.path, updateWindowTitle);
  await updateWindowTitle(state.path);
})();

refreshAll();
</script>

<template>
  <div id="container" class="h-screen border border-neutral-300 flex flex-col">
    <TitleBar class="flex-none" />
    <QueryBar />
    <div class="flex-grow"></div>
    <StatusBar class="flex-none" />
  </div>
</template>
