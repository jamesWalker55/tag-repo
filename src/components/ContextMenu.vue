<script lang="ts" setup>
import { ref } from "vue";
import { Copy, Cut, Paste } from "@/lib/icons";
import Menu from "@/components/menu/Menu.vue";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import MenuArbitraryItem from "@/components/menu/MenuArbitraryItem.vue";

const isVisible = ref(false);

const posX = ref(0);
const posY = ref(0);

function onShowMenu(e: MouseEvent) {
  posX.value = e.clientX;
  posY.value = e.clientY;
  isVisible.value = true;
}

function closeMenu() {
  isVisible.value = false;
}

defineExpose({ show: onShowMenu });

const log = console.log;
</script>

<template>
  <Teleport v-if="isVisible" to="#context-menu-container">
    <!--suppress VueUnrecognizedDirective -->
    <Menu :pos-x="posX" :pos-y="posY" v-click-away="closeMenu">
      <slot v-bind="{ closeMenu }" />
    </Menu>
  </Teleport>
</template>

<style scoped>
.menu-grid {
  grid-template-columns: max-content max-content max-content max-content;
}
</style>
