<script lang="ts" setup>
import TitleBar from "./components/toolbars/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/toolbars/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { selection } from "@/lib/api";
import { computed } from "vue";
import PanelsContainer from "@/components/PanelsContainer.vue";
import ItemProperties from "@/components/ItemProperties.vue";

// const propertiesVisible = computed(() => selection.selectedCount.value > 0);
const propertiesVisible = true;

// disable the native context menu except certain elements only
function onContextMenu(e: MouseEvent) {
  const clickedElement = e.target as Element;
  if (clickedElement.tagName !== "INPUT") {
    e.preventDefault();
  }
}
document.addEventListener("contextmenu", onContextMenu);
</script>

<template>
  <div
    id="container"
    class="app-grid relative grid h-screen max-h-screen select-none border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <QueryBar class="flex-none" />
    <PanelsContainer
      is="main"
      class="relative flex-1"
      :right-size-key="propertiesVisible ? 'rightPanel' : null"
    >
      <ItemList />
      <template #right>
        <ItemProperties />
      </template>
    </PanelsContainer>
    <StatusBar />
  </div>
</template>

<style scoped>
.app-grid {
  grid-template-rows: max-content max-content minmax(0, 1fr) max-content;
}
</style>
