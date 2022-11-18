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
        menu_class: 'rounded text-sm',
        menu: [
          {text: 'Open repository...', icon: OpenFolderIcon, click: () => alert('Action 1')},
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
        class: 'w-title-button !rounded-none',
      },
      {
        icon: MaximiseIcon,
        click: () => appWindow.toggleMaximize(),
        class: 'w-title-button !rounded-none',
      },
      {
        icon: CloseIcon,
        click: () => appWindow.close(),
        class: 'w-title-button !rounded-none',
      },
    ],
);
</script>

<template>
  <VueFileToolbarMenu
      :content="menuItems"
      class="toolbar
        !bg-neutral-100 !text-neutral-900
        !border-solid !border-0 !border-b !border-b-neutral-300"
      data-tauri-drag-region/>
</template>

<style>
.toolbar {
  --bar-font-color: rgb(32, 33, 36);
  --bar-font-size: 14px;
  //--bar-letter-spacing: 0.013333333em;
  --bar-padding: 2px;
  --bar-button-icon-size: 18px;
  --bar-button-padding: 1px 6px 1px 6px;
  --bar-button-radius: 4px;
  --bar-button-hover-bkg: rgb(241, 243, 244);
  --bar-button-active-color: rgb(26, 115, 232);
  --bar-button-active-bkg: rgb(232, 240, 254);
  --bar-button-open-color: rgb(32, 33, 36);
  --bar-button-open-bkg: rgb(232, 240, 254);
  --bar-menu-bkg: unset;
  //--bar-menu-border-radius: 0 0 3px 3px;
  //--bar-menu-item-chevron-margin: 0;
  //--bar-menu-item-hover-bkg: rgb(241, 243, 244);
  --bar-menu-item-padding: 2px 8px 2px 27px;
  --bar-menu-item-icon-size: 14px;
  --bar-menu-item-icon-margin: 0 7px 0 -21px;
  --bar-menu-padding: 4px 0px;
  --bar-menu-shadow: 0 2px 6px 2px rgba(60, 64, 67, 0.15);
  --bar-menu-separator-height: 1px;
  --bar-menu-separator-margin: 4px 8px;
  --bar-menu-separator-color: rgb(227, 229, 233);
  --bar-separator-color: rgb(218, 220, 224);
  --bar-separator-width: 1px;
  --bar-sub-menu-border-radius: 3px;
}
</style>
