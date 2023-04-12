<script lang="ts" setup>
import TitleBar from "./components/toolbars/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/toolbars/StatusBar.vue";
import ItemList from "./components/ItemList.vue";
import { selection } from "@/lib/api";
import { computed, Ref, ref } from "vue";
import PanelsContainer from "@/components/PanelsContainer.vue";
import ItemProperties from "@/components/ItemProperties.vue";
import PopupMenu from "@/components/ContextMenu.vue";
import {
  TitleMinimize,
  TitleMaximise,
  TitleClose,
  MenuClose,
  OpenRepo,
  Copy,
  Cut,
  Paste,
  AppLogo,
} from "@/lib/icons";
import Menu from "@/components/menu/Menu.vue";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import MenuArbitraryItem from "@/components/menu/MenuArbitraryItem.vue";

// const propertiesVisible = computed(() => selection.selectedCount.value > 0);
const propertiesVisible = true;

const menu = ref<InstanceType<typeof PopupMenu> | null>(null);

// disable the native context menu except certain elements only
function onContextMenu(e: MouseEvent) {
  const clickedElement = e.target as Element;
  if (clickedElement.tagName !== "INPUT") {
    e.preventDefault();
    menu.value?.show(e);
  }
}
document.addEventListener("contextmenu", onContextMenu);
const log = console.log;
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
    <Menu :pos-x="10" :pos-y="10">
      <MenuItem text="Cut" alt-text="Ctrl+X" @click="log">
        <template #icon="{ defaultClasses }"
          ><Cut :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuItem text="Copy" alt-text="Ctrl+C">
        <template #icon="{ defaultClasses }"
          ><Copy :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuItem text="Paste" alt-text="Ctrl+Shift+V">
        <template #icon="{ defaultClasses }"
          ><Paste :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuSeparator />
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown"> 1,2,3 </MenuItem>
      <MenuSeparator />
      <MenuArbitraryItem> Custom </MenuArbitraryItem>
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown" />
    </Menu>
    <Menu :pos-x="250" :pos-y="10">
      <MenuItem text="Cut" alt-text="Ctrl+X" />
      <MenuItem text="Copy" alt-text="Ctrl+C" />
      <MenuItem text="Paste" alt-text="Ctrl+Shift+V" />
      <MenuSeparator />
      <MenuItem text="Unknown" />
      <MenuSeparator />
      <MenuArbitraryItem> Custom </MenuArbitraryItem>
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown" />
    </Menu>
    <Menu :pos-x="10" :pos-y="300">
      <MenuItem text="Cut" @click="log">
        <template #icon="{ defaultClasses }"
          ><Cut :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuItem text="Copy">
        <template #icon="{ defaultClasses }"
          ><Copy :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuItem text="Paste">
        <template #icon="{ defaultClasses }"
          ><Paste :class="defaultClasses"
        /></template>
      </MenuItem>
      <MenuSeparator />
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown"> 1,2,3 </MenuItem>
      <MenuSeparator />
      <MenuArbitraryItem> Custom </MenuArbitraryItem>
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown" />
    </Menu>
    <Menu :pos-x="250" :pos-y="300">
      <MenuItem text="Cut" />
      <MenuItem text="Copy" />
      <MenuItem text="Paste" />
      <MenuSeparator />
      <MenuItem text="Unknown" />
      <MenuSeparator />
      <MenuItem text="Unknown" />
      <MenuItem text="Unknown" />
    </Menu>
  </div>
</template>

<style scoped>
.app-grid {
  grid-template-rows: max-content max-content minmax(0, 1fr) max-content;
}
</style>
