<script lang="ts" setup>
import VueFileToolbarMenu from 'vue-file-toolbar-menu';

import MinimizeIcon from '~icons/material-symbols/minimize';
import MaximiseIcon from '~icons/material-symbols/fullscreen';
import CloseIcon from '~icons/material-symbols/close';
import OpenFolderIcon from '~icons/heroicons/folder-open';
import CloseIcon2 from '~icons/heroicons/x-mark';

import {appWindow} from '@tauri-apps/api/window';
import {computed, ref} from 'vue';

const happy = ref(false);

const menuItems = computed(() => [
      {
        text: 'File',
        menu: [
          {text: 'Open repository...', icon: OpenFolderIcon, click: () => alert('Action 1')},
          {is: 'separator'},
          {text: 'Exit', icon: CloseIcon2, click: () => appWindow.close()},
        ],
      },
      {
        text: 'Edit',
        menu: [
          {text: 'Copy'},
          {text: 'Cut'},
          {text: 'Paste'},
          {is: 'separator'},
          {text: 'Exit', icon: CloseIcon2, click: () => appWindow.close()},
        ],
      },
      {
        text: 'My Button',
        active: happy.value,
        icon: happy.value ? 'sentiment_very_satisfied' : 'sentiment_satisfied',
        click: () => { happy.value = !happy.value; },
      },
      // Spacer
      {is: 'div', class: 'ml-auto'},
      {
        icon: MinimizeIcon,
        click: () => appWindow.minimize(),
        class: 'title-button',
      },
      {
        icon: MaximiseIcon,
        click: () => appWindow.toggleMaximize(),
        class: 'title-button',
      },
      {
        icon: CloseIcon,
        click: () => appWindow.close(),
        class: 'title-button',
      },
    ],
);
</script>

<template>
  <VueFileToolbarMenu
      id="toolbar"
      :content="menuItems"
      data-tauri-drag-region/>
</template>

<style scoped>
// i'm using a random #toolbar ID to make these styles more important, so it overrides the default styles
#toolbar.bar {
  // Text styling
  @apply text-sm;

  // Button colors
  @apply text-neutral-900;

  :deep(.bar-button) {
    @apply hover:bg-neutral-100 transition-colors duration-75;
    &:active {@apply bg-neutral-300;}
  }

  :deep(.bar-button.active) {
    @apply bg-emerald-100 hover:bg-emerald-200 text-emerald-600;
    &:active {@apply bg-emerald-300;}
  }

  :deep(.bar-button.open) {
    @apply hover:bg-neutral-300 text-neutral-900;
  }

  // Button styling
  :deep(.bar-button) {
    @apply px-1 py-0 rounded-none;
  }
  :deep(.bar-button.title-button) {
    @apply w-title-button;
  }

  // Button icon sizes
  :deep(.bar-button) {
    .icon {@apply w-5;}
    .material-icons.icon {@apply text-xl;}
  }
}
</style>
