<script lang="ts" setup>
import VueFileToolbarMenu from "vue-file-toolbar-menu";

import {
  TitleMinimize,
  TitleMaximise,
  TitleClose,
  MenuClose,
  OpenRepo,
  FaceSmile,
  FaceFrown,
  Copy,
  Cut,
  Paste,
} from "@/lib/icons";

import { appWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api";
import { computed, ref } from "vue";

const happy = ref(false);

const menuItems = computed(() => [
  {
    text: "File",
    menu: [
      {
        text: "Open repository...",
        icon: OpenRepo,
        click: () => alert("Action 1"),
      },
      { is: "separator" },
      { text: "Exit", icon: MenuClose, click: () => appWindow.close() },
    ],
  },
  {
    text: "Edit",
    menu: [
      { text: "Cut", icon: Cut },
      { text: "Copy", icon: Copy },
      { text: "Paste", icon: Paste },
      { is: "separator" },
      { text: "Tools", disabled: true },
    ],
  },
  {
    active: happy.value,
    icon: happy.value ? FaceFrown : FaceSmile,
    click: () => {
      happy.value = !happy.value;
    },
  },
  {
    icon: OpenRepo,
    click: () => {
      happy.value = !happy.value;
    },
  },
  // Spacer
  { is: "div", class: "ml-auto" },
  {
    icon: TitleMinimize,
    click: () => appWindow.minimize(),
    class: "title-button",
  },
  {
    icon: TitleMaximise,
    click: () => appWindow.toggleMaximize(),
    class: "title-button",
  },
  {
    icon: TitleClose,
    click: () => appWindow.close(),
    class: "title-button danger",
  },
]);
</script>

<template>
  <VueFileToolbarMenu
    id="toolbar"
    :content="menuItems"
    data-tauri-drag-region
  />
</template>

<style scoped>
// i'm using a random #toolbar ID to make these styles more important, so it overrides the default styles
#toolbar.bar {
  // Text styling
  @apply text-sm;

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
      @apply w-5;
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
