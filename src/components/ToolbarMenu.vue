<script lang="ts" setup>
import { computed, Ref, ref, watch } from "vue";
import Menu from "@/components/menu/Menu.vue";

const isVisible = ref(false);

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

// dimensions of the element that initiated the menu
// i.e. values from getBoundingClientRect() of the menu button you clicked
const initiatorRect: Ref<Rect> = ref({ x: 0, y: 0, width: 0, height: 0 });

const menuWidth = ref(0);
const menuHeight = ref(0);

const menuSizeKnown = computed(
  () => menuWidth.value !== 0 && menuHeight.value !== 0
);

const bestX = computed(() => {
  const windowWidth = window.innerWidth;
  const buttonRect = initiatorRect.value;

  // try to put the menu at clicked position
  if (buttonRect.x + menuWidth.value < windowWidth) {
    return buttonRect.x;
  }

  // try to put it backwards
  if (buttonRect.x + buttonRect.width - menuWidth.value >= 0) {
    return buttonRect.x + buttonRect.width - menuWidth.value;
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
  const buttonRect = initiatorRect.value;

  // try to put the menu at clicked position
  if (buttonRect.y + buttonRect.height + menuHeight.value < windowHeight) {
    return buttonRect.y + buttonRect.height;
  }

  // try to put it backwards
  if (buttonRect.y - menuHeight.value >= 0) {
    return buttonRect.y - menuHeight.value;
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
  const clickedElement = e.target as Element;
  const rect = clickedElement.getBoundingClientRect();
  initiatorRect.value = {
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
  };
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
