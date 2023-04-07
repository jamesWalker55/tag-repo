<script lang="ts" setup>
import VueFileToolbarMenu from "vue-file-toolbar-menu";

import { Feedback } from "@/lib/icons";
import FeedbackModal from "./FeedbackModal.vue";
import { appWindow } from "@tauri-apps/api/window";
import { computed, ref } from "vue";
import * as api from "@/lib/api";
import StatusBarInfo from "./StatusBarInfo.vue";

const feedbackPopup = ref(false);

const menuItems = computed(() => [
  {
    is: StatusBarInfo,
  },
  {
    icon: Feedback,
    click: () => (feedbackPopup.value = !feedbackPopup.value),
  },
]);
</script>

<template>
  <VueFileToolbarMenu
    id="toolbar"
    :content="menuItems"
    data-tauri-drag-region
  />
  <FeedbackModal
    v-if="feedbackPopup"
    @closed="feedbackPopup = !feedbackPopup"
  />
</template>

<style scoped>
// i'm using a random #toolbar ID to make these styles more important, so it overrides the default styles
#toolbar.bar {
  // Toolbar styling
  //
  // The structure of the toolbar is as follows:
  // .bar
  //   .bar-button
  //     .label / .icon (name or icon of the button)
  //     .bar-menu.menu (the popup menu, hidden by default)
  //       ...
  //   .bar-button
  //   (each menu item)
  //   ...

  @apply border-x-0 border-b-0 border-t border-solid border-neutral-200 bg-neutral-50;
  @apply mx-2;

  // Text styling
  @apply text-sm;

  // Limit toolbar to a single row
  @apply flex;
  @apply flex-nowrap;
  @apply items-center;

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
    @apply rounded-none px-0.5 py-0;
  }

  // Button icon sizes
  :deep(.bar-button) {
    .icon {
      @apply h-5 w-4;
    }
    .material-icons.icon {
      @apply text-lg;
    }
  }

  // Menu styling
  //
  // The structure of the menu is as follows:
  // .bar-menu
  //   .extended-hover-zone (hidden)
  //   .bar-menu-items (container for items)
  //     .bar-menu-item (a menu item)
  //       .icon (optional icon)
  //       .label (text for the item)
  //       .hotkey (optional hotkey text)
  //     .bar-menu-separator (a menu separator)
  //     ...
  :deep(.bar-menu-items) {
    @apply text-neutral-900;
    @apply px-0 py-1;
    @apply shadow-none drop-shadow-md;
  }

  :deep(.bar-menu-item) {
    @apply w-60 py-0.5 pl-8 pr-3 transition-colors duration-75 ease-out hover:bg-neutral-100;
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
