<script lang="ts" setup>
import { computed, Ref, ref, watch } from "vue";
import Menu from "@/components/menu/Menu.vue";

const isVisible = ref(false);

const clickedX = ref(0);
const clickedY = ref(0);

const menuWidth = ref(0);
const menuHeight = ref(0);

const menuSizeKnown = computed(
  () => menuWidth.value !== 0 && menuHeight.value !== 0
);

const bestX = computed(() => {
  const windowWidth = window.innerWidth;

  // try to put the menu at clicked position
  if (clickedX.value + menuWidth.value < windowWidth) {
    return clickedX.value;
  }

  // try to put it backwards
  if (clickedX.value - menuWidth.value >= 0) {
    return clickedX.value - menuWidth.value;
  }

  // if it fit in the screen, offset it
  if (menuWidth.value <= windowWidth) {
    return windowWidth - menuWidth.value;
  }

  // it doesn't fit in the screen, just put at 0
  return 0;
});

const bestY = computed(() => {
  const windowHeight = window.innerHeight;

  // try to put the menu at clicked position
  if (clickedY.value + menuHeight.value < windowHeight) {
    return clickedY.value;
  }

  // try to put it backwards
  if (clickedY.value - menuHeight.value >= 0) {
    return clickedY.value - menuHeight.value;
  }

  // if it fit in the screen, offset it
  if (menuHeight.value <= windowHeight) {
    return windowHeight - menuHeight.value;
  }

  // it doesn't fit in the screen, just put at 0
  return 0;
});

function onReceiveMenuSize(width: number, height: number) {
  menuWidth.value = width;
  menuHeight.value = height;
}

function showMenu(e: MouseEvent) {
  clickedX.value = e.clientX;
  clickedY.value = e.clientY;
  isVisible.value = true;
}

function closeMenu() {
  isVisible.value = false;
}

defineExpose({ show: showMenu });

const log = console.log;
</script>

<template>
  <Teleport v-if="isVisible" to="#context-menu-container">
    <!--suppress VueUnrecognizedDirective -->
    <Menu
      ref="menu"
      :pos-x="menuSizeKnown ? bestX : 0"
      :pos-y="menuSizeKnown ? bestY : 0"
      v-click-away="closeMenu"
      @resized="onReceiveMenuSize"
      :class="menuSizeKnown ? '' : '-z-10 opacity-0'"
    >
      <slot v-bind="{ closeMenu }" />
    </Menu>
  </Teleport>
</template>

<style scoped>
.menu-grid {
  grid-template-columns: max-content max-content max-content max-content;
}
</style>
