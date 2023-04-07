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
  <div
    id="container"
    class="relative flex h-screen select-none flex-col border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <main class="relative flex flex-1 flex-col">
      <QueryBar />
    </main>
    <StatusBar class="flex-none" />
  </div>
</template>
