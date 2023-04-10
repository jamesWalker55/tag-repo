<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { selection, state } from "@/lib/api";
import { computed } from "vue";
import BottomPanel from "@/components/BottomPanel.vue";
import { PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH } from "@/lib/constants";
import RightPanel from "@/components/RightPanel.vue";
import LeftPanel from '@/components/LeftPanel.vue';

const propertiesVisible = computed(() => selection.selectedCount.value > 0);

const mainGridRows = computed(() => {
  const lowerBoundedPanelHeight = Math.max(
    PANEL_MIN_HEIGHT,
    state.panelSizes["bottomPanel"]
  );
  const rows = [
    "minmax(0, 1fr)",
    `minmax(auto, ${lowerBoundedPanelHeight}px)`,
  ];

  return rows.join(" ");
});

const mainGridCols = computed(() => {
  const lowerBoundedLeftPanelWidth = Math.max(
    PANEL_MIN_WIDTH,
    state.panelSizes["leftPanel"],
  );
  const lowerBoundedRightPanelWidth = Math.max(
    PANEL_MIN_WIDTH,
    state.panelSizes["rightPanel"],
  );
  const rows = [
    `minmax(auto, ${lowerBoundedLeftPanelWidth}px)`,
    "minmax(0, 1fr)",
    `minmax(auto, ${lowerBoundedRightPanelWidth}px)`,
  ];

  return rows.join(" ");
});
</script>

<template>
  <div
    id="container"
    class="app-grid relative grid h-screen max-h-screen select-none border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <QueryBar class="flex-none" />
    <main class="main-grid relative grid">
      <ItemList style="grid-area: m" />
      <BottomPanel size-key="bottomPanel" style="grid-area: b">
        bottom content
      </BottomPanel>
      <LeftPanel size-key="leftPanel" style="grid-area: l">
        side content
      </LeftPanel>
      <RightPanel size-key="rightPanel" style="grid-area: r">
        side content
      </RightPanel>
    </main>
    <StatusBar />
  </div>
</template>

<style scoped>
.app-grid {
  grid-template-rows: max-content max-content minmax(0, 1fr) max-content;
}
.main-grid {
  grid-template-rows: v-bind("mainGridRows");
  grid-template-columns: v-bind("mainGridCols");
  grid-template-areas:
    "l m r"
    "l b b";
}
</style>
