<script lang="ts" setup>
import ToolbarMenu from "@/components/ToolbarMenu.vue";
import MenuArbitraryItem from "@/components/menu/MenuArbitraryItem.vue";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import MenuText from "@/components/menu/MenuText.vue";
import AudioSlider from "@/components/toolbars/AudioSlider.vue";
import ToolbarButton from "@/components/toolbars/ToolbarButton.vue";
import * as api from "@/lib/api";
import { openManual, selection, state } from "@/lib/api";
import {
  shuffleList,
  toggleLeftPanelVisibility,
  toggleRightPanelVisibility,
} from "@/lib/api/actions";
import { setAudioVolume, toggleAudioPreview } from "@/lib/api/audio-preview";
import { updateWindowSizeConfig } from "@/lib/ffi";
import { appWindow } from "@tauri-apps/api/window";
import { Ref, ref } from "vue";

type ToolbarMenuType = InstanceType<typeof ToolbarMenu>;

const fileMenu: Ref<ToolbarMenuType | null> = ref(null);
const editMenu: Ref<ToolbarMenuType | null> = ref(null);
const previewMenu: Ref<ToolbarMenuType | null> = ref(null);
const viewMenu: Ref<ToolbarMenuType | null> = ref(null);
const helpMenu: Ref<ToolbarMenuType | null> = ref(null);

const allMenuRefs = [fileMenu, editMenu, previewMenu, viewMenu, helpMenu];

const anyMenuActive = ref(false);

function onButtonClick(e: MouseEvent, menu: ToolbarMenuType | null) {
  menu?.show(e);
  anyMenuActive.value = true;
}

function onButtonClickAway(e: MouseEvent, menu: ToolbarMenuType | null) {
  anyMenuActive.value = false;
}

function onButtonMouseOver(e: MouseEvent, menu: ToolbarMenuType | null) {
  if (!anyMenuActive.value) return;

  // close all other menus, while keeping the given menu open
  for (let menuRef of allMenuRefs) {
    if (menuRef.value === menu) {
      menu?.show(e);
    } else {
      menuRef.value?.close();
    }
  }
}
</script>

<template>
  <div class="flex h-7 min-w-0 flex-row items-center" data-tauri-drag-region>
    <!-- App logo -->
    <i-fluent-tag-20-filled
      height="20"
      width="20"
      class="mx-1 flex-none text-orange-500"
      data-tauri-drag-region
    />

    <!-- File menu -->
    <ToolbarButton
      @click="(e) => onButtonClick(e, fileMenu)"
      v-click-away="(e: MouseEvent) => onButtonClickAway(e, fileMenu)"
      @mouseover.self="(e) => onButtonMouseOver(e, fileMenu)"
    >
      File
    </ToolbarButton>
    <ToolbarMenu ref="fileMenu" v-slot="{ closeMenu }">
      <MenuItem
        text="Open repository..."
        @click="
          () => {
            api.promptOpenRepo();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <i-fluent-folder-open-16-regular
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
      <MenuItem
        text="Close repository"
        @click="
          () => {
            api.closeRepo();
            closeMenu();
          }
        "
      />
      <MenuSeparator />
      <MenuItem
        text="Exit"
        @click="() => updateWindowSizeConfig().then(() => appWindow.close())"
      >
        <template #icon="{ defaultClasses }">
          <i-fluent-dismiss-16-regular
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
    </ToolbarMenu>

    <!-- Edit menu -->
    <ToolbarButton
      @click="(e) => onButtonClick(e, editMenu)"
      v-click-away="(e: MouseEvent) => onButtonClickAway(e, editMenu)"
      @mouseover.self="(e) => onButtonMouseOver(e, editMenu)"
    >
      Edit
    </ToolbarButton>
    <ToolbarMenu ref="editMenu" v-slot="{ closeMenu }">
      <MenuItem
        text="Clear Selection"
        @click="
          () => {
            if (selection.selectedCount.value !== 0) {
              selection.clear();
            }
            closeMenu();
          }
        "
        :disabled="selection.selectedCount.value === 0"
      />
      <MenuSeparator />
      <MenuText>Tools</MenuText>
      <MenuItem
        text="Shuffle results"
        @click="
          () => {
            shuffleList();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <i-fluent-arrow-sync-16-regular
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
    </ToolbarMenu>

    <!-- View menu -->
    <ToolbarButton
      @click="(e) => onButtonClick(e, viewMenu)"
      v-click-away="(e: MouseEvent) => onButtonClickAway(e, viewMenu)"
      @mouseover.self="(e) => onButtonMouseOver(e, viewMenu)"
    >
      View
    </ToolbarButton>
    <ToolbarMenu ref="viewMenu">
      <MenuText>Panels</MenuText>
      <MenuItem text="Item Properties" @click="toggleRightPanelVisibility">
        <template #icon="{ defaultClasses }">
          <i-fluent-checkbox-checked-16-regular
            v-if="state.panelVisibility.rightPanel"
            width="16"
            height="16"
            :class="defaultClasses"
          />
          <i-fluent-checkbox-unchecked-16-regular
            v-else
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
      <MenuItem text="Folders" @click="toggleLeftPanelVisibility">
        <template #icon="{ defaultClasses }">
          <i-fluent-checkbox-checked-16-regular
            v-if="state.panelVisibility.leftPanel"
            width="16"
            height="16"
            :class="defaultClasses"
          />
          <i-fluent-checkbox-unchecked-16-regular
            v-else
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
    </ToolbarMenu>

    <!-- Preview menu -->
    <ToolbarButton
      @click="(e) => onButtonClick(e, previewMenu)"
      v-click-away="(e: MouseEvent) => onButtonClickAway(e, previewMenu)"
      @mouseover.self="(e) => onButtonMouseOver(e, previewMenu)"
    >
      Audio
    </ToolbarButton>
    <ToolbarMenu ref="previewMenu">
      <MenuItem text="Audio preview" @click="toggleAudioPreview()">
        <template #icon="{ defaultClasses }">
          <i-fluent-checkbox-checked-16-regular
            v-if="state.audioPreview"
            width="16"
            height="16"
            :class="defaultClasses"
          />
          <i-fluent-checkbox-unchecked-16-regular
            v-else
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
      <MenuSeparator />
      <MenuArbitraryItem>
        <AudioSlider
          class="h-14"
          :disabled="!state.audioPreview"
          :initial-value="state.audioVolume"
          @value-changed="(val) => setAudioVolume(val)"
        />
      </MenuArbitraryItem>
    </ToolbarMenu>

    <!-- Help menu -->
    <ToolbarButton
      @click="(e) => onButtonClick(e, helpMenu)"
      v-click-away="(e: MouseEvent) => onButtonClickAway(e, helpMenu)"
      @mouseover.self="(e) => onButtonMouseOver(e, helpMenu)"
    >
      Help
    </ToolbarButton>
    <ToolbarMenu ref="helpMenu" v-slot="{ closeMenu }">
      <MenuItem
        text="Manual"
        @click="
          (e) => {
            openManual();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <i-fluent-book-open-16-regular
            width="16"
            height="16"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
    </ToolbarMenu>

    <!-- Spacer / info -->
    <div
      class="min-w-0 flex-1 overflow-hidden truncate text-center text-sm text-neutral-400"
      data-tauri-drag-region
    >
      {{ api.state.path ? api.state.path + " - " : "" }}tagrepo
    </div>

    <!-- Window buttons -->
    <ToolbarButton class="w-title-button" @click="() => appWindow.minimize()">
      <i-fluent-subtract-16-regular width="16" height="16" />
    </ToolbarButton>
    <ToolbarButton
      class="w-title-button"
      @click="() => appWindow.toggleMaximize()"
    >
      <i-fluent-maximize-16-regular width="16" height="16" />
    </ToolbarButton>
    <ToolbarButton
      class="w-title-button hover:bg-red-600 hover:text-white active:bg-red-500 active:text-white"
      @click="() => updateWindowSizeConfig().then(() => appWindow.close())"
    >
      <i-fluent-dismiss-16-regular width="16" height="16" />
    </ToolbarButton>
  </div>
</template>
