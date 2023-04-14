<script lang="ts" setup>
import { appWindow } from "@tauri-apps/api/window";
import { Ref, ref } from "vue";
import * as api from "@/lib/api";
import { selection } from "@/lib/api";
import { shuffleList } from "@/lib/api/actions";
import ToolbarButton from "@/components/toolbars/ToolbarButton.vue";
import ToolbarMenu from "@/components/ToolbarMenu.vue";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import MenuText from "@/components/menu/MenuText.vue";

const fileMenu: Ref<InstanceType<typeof ToolbarMenu> | null> = ref(null);
const editMenu: Ref<InstanceType<typeof ToolbarMenu> | null> = ref(null);
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
    <ToolbarButton @click="fileMenu?.show">File</ToolbarButton>
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
      <MenuItem text="Exit" @click="() => appWindow.close()">
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
    <ToolbarButton @click="editMenu?.show">Edit</ToolbarButton>
    <ToolbarMenu ref="editMenu" v-slot="{ closeMenu }">
      <MenuItem
        text="Clear Selection"
        @click="
          () => {
            selection.clear();
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
      @click="() => appWindow.close()"
    >
      <i-fluent-dismiss-16-regular width="16" height="16" />
    </ToolbarButton>
  </div>
</template>
