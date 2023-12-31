<script lang="ts" setup>
import FolderTree from "@/components/FolderTree.vue";
import ItemProperties from "@/components/ItemProperties.vue";
import PanelsContainer from "@/components/PanelsContainer.vue";
import { state } from "@/lib/api";
import { Ref, ref } from "vue";
import ItemList from "./components/ItemList.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/toolbars/StatusBar.vue";
import TitleBar from "./components/toolbars/TitleBar.vue";

// disable the native context menu except certain elements only
function onContextMenu(e: MouseEvent) {
  const clickedElement = e.target as Element;
  if (clickedElement.tagName !== "INPUT") {
    e.preventDefault();
  }
}
document.addEventListener("contextmenu", onContextMenu);

const itemList: Ref<InstanceType<typeof ItemList> | null> = ref(null);
</script>

<template>
  <div
    id="container"
    class="app-grid relative grid h-screen max-h-screen select-none overflow-clip border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <QueryBar class="flex-none" @keydown.enter="itemList?.focus()" />
    <PanelsContainer
      is="main"
      class="relative flex-1"
      :left-size-key="state.panelVisibility.leftPanel ? 'leftPanel' : null"
      :right-size-key="state.panelVisibility.rightPanel ? 'rightPanel' : null"
    >
      <ItemList ref="itemList" />
      <template #left>
        <div class="grid lpanel-grid h-full">
          <FolderTree class="flex-1" />
          <div
            class="overflow-x-auto py-1 pl-0.5 text-sm flex-1 border-t border-neutral-300"
          >
            <div v-for="tag of state.tags" :key="tag.name">
              {{ tag.name }} <i>{{ tag.count }}</i>
            </div>
          </div>
        </div>
      </template>
      <template #right>
        <ItemProperties />
      </template>
    </PanelsContainer>
    <StatusBar />
    <div id="context-menu-container"></div>
  </div>
</template>

<style scoped>
.app-grid {
  grid-template-rows: max-content max-content minmax(0, 1fr) max-content;
}

.lpanel-grid {
  grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
}
</style>
