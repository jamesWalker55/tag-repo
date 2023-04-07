<script lang="ts" setup>
import VueFileToolbarMenu from "vue-file-toolbar-menu";

import { Feedback } from "@/lib/icons";

import { appWindow } from "@tauri-apps/api/window";
import { computed, ref } from "vue";
import * as api from "@/lib/api";
import TitleBarSpacer from "./TitleBarSpacer.vue";

const happy = ref(false);

const menuItems = computed(() => [
  // Right side
  {
    icon: Feedback,
    click: () => appWindow.minimize(),
  },
]);
</script>

<template>
  <VueFileToolbarMenu
    id="toolbar"
    :content="menuItems"
    class="bg-neutral-300"
    data-tauri-drag-region
  />
</template>

<style scoped>
// i'm using a random #toolbar ID to make these styles more important, so it overrides the default styles
#toolbar.bar {
  // Text styling
  @apply text-sm;

  // Limit toolbar to a single row
  @apply flex-nowrap;

  // Button colors
  @apply text-neutral-900;

  :deep(.bar-button) {
    @apply transition-colors duration-75 ease-out hover:bg-neutral-200;
    &:active {
      @apply bg-neutral-400;
    }

    &.active {
      @apply bg-emerald-100 text-emerald-600 hover:bg-emerald-200;
      &:active {
        @apply bg-emerald-300;
      }
    }

    &.open {
      @apply text-neutral-900 hover:bg-neutral-400;
    }
  }

  // Button styling
  :deep(.bar-button) {
    @apply rounded-none px-1 py-0;
  }

  // Special buttons
  :deep(.bar-button.title-button) {
    @apply w-title-button;
    @apply flex-none;

    .icon {
      @apply h-min w-4;
    }
    .material-icons.icon {
      @apply text-base;
    }

    &.danger {
      @apply hover:bg-red-600 hover:text-white;
      &:active {
        @apply bg-red-500 text-white;
      }
    }
  }

  // Button icon sizes
  :deep(.bar-button) {
    .icon {
      @apply w-4 h-5;
    }
    .material-icons.icon {
      @apply text-xl;
    }
  }

  // Menu styling
  :deep(.bar-menu-items) {
    @apply text-neutral-900;
    @apply px-0 py-1;
    @apply shadow-none drop-shadow-md;
  }

  :deep(.bar-menu-item) {
    @apply py-0.5 pl-8 pr-3 transition-colors duration-75 ease-out hover:bg-neutral-100;
    &:active {
      @apply bg-neutral-300;
    }
    &.disabled {
      @apply text-neutral-300;
    }
  }

  :deep(.bar-menu-separator) {
    @apply mx-2 my-1 h-px bg-neutral-200;
  }

  // Menu icon sizes
  :deep(.bar-menu-items) {
    .icon {
      @apply -ml-6 mr-2 h-min w-4;
    }
    .material-icons.icon {
      @apply text-base;
    }
  }
}
</style>
