<script lang="ts" setup>
import { ref } from "vue";
import { Component as TypeComponent } from "@vue/runtime-core";
import { getEmSizeInPx, parseRemSize } from "@/lib/utils";
import { MenuMore } from "@/lib/icons";

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

interface BaseItem {
  type: string;
}

interface StandardItem extends BaseItem {
  type: "item";
  text: string;
  altText?: string;
  icon?: TypeComponent;
  onClick?: (e: MouseEvent) => void;
  disabled?: boolean;
  subItems?: MenuItem[];
}

interface Separator extends BaseItem {
  type: "separator";
}

interface CustomItem extends BaseItem {
  type: "custom";
  content: TypeComponent;
}

export type MenuItem = StandardItem | Separator | CustomItem;

defineProps<{ items: MenuItem[] }>();

defineExpose({ show: onShowMenu });

const ICON_WIDTH = parseRemSize("4rem");
</script>

<template>
  <Teleport v-if="isVisible" to="body">
    <!--suppress VueUnrecognizedDirective -->
    <div
      class="menu-grid absolute flex grid flex-col items-center justify-items-stretch rounded border border-neutral-300 bg-white text-sm drop-shadow-lg"
      :style="{ left: posX + 'px', top: posY + 'px' }"
      v-click-away="closeMenu"
    >
      <template v-for="item in items">
        <template v-if="item.type === 'item'" class="px-2 py-2">
          <Component
            v-if="item.icon"
            :is="item.icon"
            class="ml-3 text-base text-neutral-600"
          />
          <div v-else></div>
          <div class="mx-3 flex h-8 items-center">{{ item.text }}</div>
          <div class="ml-2 mr-3 text-right">{{ item.altText }}</div>
          <div v-if="item.subItems?.length > 0" class="mr-3"><MenuMore /></div>
          <div v-else></div>
        </template>
        <div
          v-else-if="item.type === 'separator'"
          class="col-span-full h-px bg-neutral-300"
        ></div>
        <div v-else-if="item.type === 'custom'" class="col-span-full px-2 py-2">
          <Component :is="item.content"/>
        </div>
        <div v-else>UNSUPPORTED</div>
      </template>
    </div>
  </Teleport>
</template>

<style scoped>
.menu-grid {
  grid-template-columns: max-content max-content max-content max-content;
}
</style>
