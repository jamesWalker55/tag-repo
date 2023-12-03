<script lang="ts" setup>
import FolderTreeItem from "@/components/FolderTreeItem.vue";
import ToolbarMenu from "@/components/ToolbarMenu.vue";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import { Folder, config, getFolders, setQuery, state } from "@/lib/api";
import {
  CheckBoxChecked,
  CheckBoxUnchecked,
  ClosePanelIcon,
  DirTreeIcon,
  RefreshIcon,
  Spinner,
  VerticalDots,
} from "@/lib/icons";
import { Ref, ref, watch } from "vue";

const rootFolder: Ref<Folder | null> = ref(null);

const lastAddedPath: Ref<string | null> = ref(null);

watch(
  () => state.recursiveTree,
  (newRecursiveMode) => {
    if (lastAddedPath.value !== null) {
      addToQuery(lastAddedPath.value);
    }
  },
);

async function fetchFolders() {
  if (state.path !== null) {
    rootFolder.value = await getFolders();
  } else {
    rootFolder.value = {};
  }
}

fetchFolders().then();

watch(() => state.path, fetchFolders);

function rootFoldersCount(folder: Folder): number {
  return Object.keys(folder).length;
}

function sortedFolder(folder: Folder): [string, Folder][] {
  return Object.entries(folder).sort();
}

function addToQuery(path: string) {
  lastAddedPath.value = path;

  // TODO: For now I'll just replace the query
  if (state.recursiveTree) {
    setQuery(`in:"${path}"`);
  } else {
    setQuery(`children:"${path}"`);
  }
}

const menu = ref<InstanceType<typeof ToolbarMenu> | null>(null);

const log = console.log;
</script>

<template>
  <div class="tree-grid grid h-full">
    <!-- header bar -->
    <div
      class="flex h-6 min-w-0 flex-row items-center gap-2 border-b border-neutral-300 px-2"
    >
      <DirTreeIcon class="h-[20px] w-[20px] flex-none text-neutral-600" />
      <div class="min-w-0 flex-1 truncate whitespace-nowrap">Folders</div>
      <VerticalDots
        class="box-content h-16px w-16px flex-none cursor-pointer rounded p-0.5 text-neutral-600 hover:bg-neutral-200 hover:text-neutral-800"
        @click="(e: MouseEvent) => menu?.show(e)"
      />
    </div>
    <!-- the tree -->
    <div v-if="rootFolder !== null" class="overflow-x-auto py-1 pl-0.5 text-sm">
      <div v-if="rootFoldersCount(rootFolder) > 0">
        <FolderTreeItem
          v-for="[name, children] in sortedFolder(rootFolder)"
          :key="name"
          :name="name"
          :children="children"
          @add-to-query="addToQuery"
        />
      </div>
      <div v-else class="px-1 text-neutral-500">
        No folders in this repository.
      </div>
    </div>
    <div v-else class="flex items-center justify-center">
      <Spinner class="animate-spin text-4xl text-neutral-400" />
    </div>
    <ToolbarMenu ref="menu" v-slot="{ closeMenu }">
      <MenuItem
        text="Refresh"
        @click="
          () => {
            rootFolder = null;
            fetchFolders();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <RefreshIcon class="h-16px w-16px" :class="defaultClasses" />
        </template>
      </MenuItem>
      <MenuItem
        text="Recursive"
        @click="
          (e) => {
            state.recursiveTree = !state.recursiveTree;
            closeMenu();
            config.setFolderTree().then(config.save);
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <CheckBoxChecked v-if="state.recursiveTree" :class="defaultClasses" />
          <CheckBoxUnchecked
            v-else
            class="h-16px w-16px"
            :class="defaultClasses"
          />
        </template>
      </MenuItem>
      <MenuSeparator />
      <MenuItem text="Close" @click="log">
        <template #icon="{ defaultClasses }">
          <ClosePanelIcon class="h-16px w-16px" :class="defaultClasses" />
        </template>
      </MenuItem>
    </ToolbarMenu>
  </div>
</template>

<style scoped>
.tree-grid {
  grid-template-rows: max-content minmax(0, 1fr);
}
</style>
