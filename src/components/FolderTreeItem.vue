<script lang="ts" setup>
import { Folder } from "@/lib/api";
import { computed, ref } from "vue";
import { TreeMinimisedIcon, TreeExpandedIcon } from "@/lib/icons";
import path from "path-browserify";

interface Props {
  name: string;
  children: Folder;
}
const props = defineProps<Props>();

interface Emits {
  (e: "addToQuery", path: string): void;
}

const emit = defineEmits<Emits>();

const expanded = ref(false);

const hasChildren = computed(() => Object.keys(props.children).length > 0);

function sortedFolder(folder: Folder): [string, Folder][] {
  return Object.entries(folder).sort();
}
</script>

<template>
  <div class="flex h-6 flex-row items-center">
    <template v-if="hasChildren">
      <TreeExpandedIcon
        v-if="expanded"
        class="box-content h-[12px] w-[12px] flex-none rounded p-0.5 text-neutral-300 hover:text-neutral-700"
        @click="() => (expanded = !expanded)"
      />
      <TreeMinimisedIcon
        v-else
        class="box-content h-[12px] w-[12px] flex-none rounded p-0.5 text-neutral-300 hover:text-neutral-700"
        @click="() => (expanded = !expanded)"
      />
    </template>
    <template v-else>
      <!-- random padding -->
      <div class="box-content h-[12px] w-[12px] flex-none p-0.5"></div>
    </template>
    <div
      class="cursor-pointer whitespace-nowrap rounded px-1 py-0.5 hover:bg-neutral-200"
      @click.prevent.stop="emit('addToQuery', name)"
    >
      {{ name }}
    </div>
  </div>
  <div class="ml-3" v-if="expanded">
    <div v-for="[subname, subchildren] in sortedFolder(children)">
      <FolderTreeItem
        :name="subname"
        :children="subchildren"
        @add-to-query="
          (subpath) => emit('addToQuery', path.join(name, subpath))
        "
      />
    </div>
  </div>
</template>
