<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { selection, state } from "@/lib/api";
import { computed } from "vue";
import PropertiesPanel from "@/components/PropertiesPanel.vue";
import { PANEL_MIN_HEIGHT } from "@/lib/constants";

const propertiesVisible = computed(() => selection.selectedCount.value > 0);

const mainGridRows = computed(() => {
  const rows = [
    // the query bar
    "max-content",
    // the item list
    "minmax(0, 1fr)",
  ];

  if (propertiesVisible.value) {
    // the tagger panel
    const lowerBoundedPanelHeight = Math.max(
      PANEL_MIN_HEIGHT,
      state.propertiesPanelHeight
    );
    rows.push(`minmax(auto, ${lowerBoundedPanelHeight}px)`);
  }

  return rows.join(" ");
});
</script>

<template>
  <div
    id="container"
    class="relative grid h-screen max-h-screen select-none grid-rows-app border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <main class="main-grid relative grid">
      <QueryBar />
      <ItemList />
      <PropertiesPanel v-if="propertiesVisible" />
    </main>
    <StatusBar />
  </div>
</template>

<style scoped>
.main-grid {
  grid-template-rows: v-bind("mainGridRows");
}
</style>
