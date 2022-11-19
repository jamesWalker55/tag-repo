<script lang="ts" setup>
import VueFileToolbarMenu from 'vue-file-toolbar-menu';

import {TitleMinimize, TitleMaximise, TitleClose, MenuClose, OpenRepo, Satisfied, VerySatisfied} from '@/lib/icons';

import {appWindow} from '@tauri-apps/api/window';
import {computed, ref} from 'vue';

const happy = ref(false);

const menuItems = computed(() => [
      {
        text: 'File',
        menu: [
          {text: 'Open repository...', icon: OpenRepo, click: () => alert('Action 1')},
          {is: 'separator'},
          {text: 'Exit', icon: MenuClose, click: () => appWindow.close()},
        ],
      },
      {
        text: 'Edit',
        menu: [
          {text: 'Copy'},
          {text: 'Cut'},
          {text: 'Paste'},
          {is: 'separator'},
          {text: 'Exit', icon: VerySatisfied, click: () => appWindow.close(), disabled: true},
          {text: 'Exit', icon: VerySatisfied, click: () => appWindow.close()},
          {text: 'Exit', icon: VerySatisfied, click: () => appWindow.close()},
        ],
      },
      {
        text: 'My Button',
        active: happy.value,
        icon: happy.value ? VerySatisfied : Satisfied,
        click: () => { happy.value = !happy.value; },
      },
      // Spacer
      {is: 'div', class: 'ml-auto'},
      {
        icon: TitleMinimize,
        click: () => appWindow.minimize(),
        class: 'title-button',
      },
      {
        icon: TitleMaximise,
        click: () => appWindow.toggleMaximize(),
        class: 'title-button',
      },
      {
        icon: TitleClose,
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
    @apply hover:bg-neutral-100 transition-colors duration-75 ease-out;
    &:active {@apply bg-neutral-300;}

    &.active {
      @apply bg-emerald-100 hover:bg-emerald-200 text-emerald-600;
      &:active {@apply bg-emerald-300;}
    }

    &.open {
      @apply hover:bg-neutral-300 text-neutral-900;
    }
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

  // Menu styling
  :deep(.bar-menu-items) {
    @apply text-neutral-900;
    @apply px-0 py-1;
    @apply shadow-none drop-shadow-md;
  }

  :deep(.bar-menu-item) {
    @apply pl-8 pr-3 py-0.5 hover:bg-neutral-100 transition-colors duration-75 ease-out;
    &:active {@apply bg-neutral-300;}
    &.disabled {@apply text-neutral-300;}
  }

  :deep(.bar-menu-separator) {
    @apply h-px mx-2 my-1 bg-neutral-200;
  }

  // Menu icon sizes
  :deep(.bar-menu-items) {
    .icon {@apply w-4 mr-2 -ml-6 h-min;}
    .material-icons.icon {@apply text-base;}
  }
}
</style>
