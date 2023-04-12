<script lang="ts" setup>
import { FileType, insertTags, ItemDetails } from "@/lib/ffi";
import { computed, ComputedRef, ref, Ref, watch } from "vue";
import { requestItemToBeFetched, selection, state } from "@/lib/api";
import ItemIcon from "@/components/itemlist/ItemIcon.vue";
import LoadingDots from "@/components/LoadingDots.vue";
import { Spinner, FTMultiple, VerticalDots, AddTags } from "@/lib/icons";
import Tag from "@/components/Tag.vue";
import path from "path-browserify";

const items = computed(() =>
  selection.selected.value.map((index) => {
    const itemId = state.itemIds[index];
    requestItemToBeFetched(itemId);
    return state.itemCache[itemId];
  })
);

const itemCount = computed(() => items.value.length);

const allItemsLoaded = computed(() => {
  for (const item of items.value) {
    if (item === undefined) {
      return false;
    }
  }
  return true;
});

const displayedTags = computed(() => {
  if (!allItemsLoaded.value) return [];

  if (itemCount.value === 0) {
    return [];
  } else if (itemCount.value === 1) {
    // must not be undefined since `allItemsLoaded` is true
    return items.value[0]!.item.tags;
  } else {
    // must not be undefined since `allItemsLoaded` is true
    const uniqueTags = new Set(items.value.flatMap((item) => item!.item.tags));
    const sortedUniqueTags = Array.from(uniqueTags).sort();

    return sortedUniqueTags;
  }
});

const tagInputField: Ref<HTMLInputElement | null> = ref(null);
watch(tagInputField, (newField) => {
  if (newField !== null) {
    // field has just appeared
    newField.focus();
  }
});

// if this is null, then the input is invisible
// otherwise, the input is visible
const tagInputValue: Ref<string | null> = ref(null);

function onAddTagsClick() {
  if (items.value === undefined) return;
  if (tagInputValue.value === null) return;
  if (!allItemsLoaded.value) return;

  console.log("tagInputValue.value:", tagInputValue.value);

  console.log();
  insertTags(
    items.value.map((item) => item!.item.id),
    tagInputValue.value.split(/(\s+)/)
  );
  tagInputValue.value = null;
}
const log = console.log;
</script>

<template>
  <div class="flex h-full flex-col px-3 py-2">
    <!-- title bar -->
    <div class="mb-5 flex h-5 flex-none flex-row items-center gap-2">
      <!-- icon -->
      <ItemIcon
        v-if="!allItemsLoaded"
        :filetype="FileType.UNKNOWN"
        class="h-16px w-16px flex-none animate-pulse rounded-full text-neutral-400"
      />
      <ItemIcon
        v-else-if="itemCount === 0"
        :filetype="FileType.UNKNOWN"
        class="h-16px w-16px flex-none text-neutral-500"
      />
      <!-- must not be undefined since allItemsLoaded === true -->
      <ItemIcon
        v-else-if="itemCount === 1"
        :filetype="items[0]!.filetype"
        class="h-16px w-16px flex-none text-neutral-600"
      />
      <FTMultiple v-else class="flex-0 h-16px w-16px text-neutral-600" />
      <!-- text -->
      <span
        is="div"
        v-if="!allItemsLoaded"
        class="h-4 flex-1 animate-pulse rounded-full bg-neutral-100 italic"
      />
      <span
        v-else-if="itemCount === 0"
        class="min-w-0 flex-1 truncate whitespace-nowrap italic text-neutral-400"
      >
        No item selected
      </span>
      <span
        v-else-if="itemCount === 1"
        class="min-w-0 flex-1 truncate whitespace-nowrap"
      >
        <!-- must not be undefined since allItemsLoaded === true -->
        {{ items[0]!.item.path }}
      </span>
      <span v-else class="min-w-0 flex-1 truncate whitespace-nowrap">
        Multiple items
      </span>
      <!-- button -->
      <VerticalDots class="h-16px w-16px flex-none" />
    </div>
    <!-- tags list -->
    <div class="flex flex-1 flex-col">
      <div class="font-bold text-neutral-500">
        <template v-if="allItemsLoaded && itemCount > 1">
          Common Tags
        </template>
        <template v-else>Tags</template>
      </div>
      <div v-if="!allItemsLoaded" class="animate-pulse italic text-neutral-400">
        Loading<LoadingDots />
      </div>
      <div
        v-else-if="displayedTags.length === 0"
        class="italic text-neutral-400"
      >
        No tags
      </div>
      <div v-else>
        <template v-for="tag in displayedTags">
          <!-- must not be undefined since allItemsLoaded === true -->
          <Tag :name="tag" :item-id="items.map((i) => i!.item.id)" />
          {{ " " }}
        </template>
      </div>
      <template v-if="itemCount > 0">
        <div
          v-if="tagInputValue === null"
          class="mt-2 flex h-6 cursor-pointer flex-row items-center gap-1 text-base text-neutral-400 hover:text-neutral-700 hover:underline hover:decoration-dotted"
          @click="tagInputValue = ''"
        >
          <AddTags class="flex-none" />
          <span class="truncate whitespace-nowrap">Add tags</span>
        </div>
        <div v-else class="mt-2 flex h-6 flex-row items-center gap-1 text-base">
          <input
            type="text"
            ref="tagInputField"
            v-model="tagInputValue"
            class="flex-1 rounded px-2 py-0.5 text-neutral-700 outline outline-1 outline-slate-300"
            @keydown.enter="onAddTagsClick"
            @keydown.esc="
              (e) => {
                tagInputValue = null;
                log(e);
              }
            "
          />
          <AddTags
            class="cursor-pointer text-neutral-400 hover:text-neutral-700"
            @click="onAddTagsClick"
          />
        </div>
      </template>
    </div>
    <!-- properties -->
    <div class="flex-none">
      <div class="font-bold text-neutral-600">Properties</div>
      <div v-if="!allItemsLoaded || itemCount === 0">
        <div class="italic text-neutral-400">None</div>
      </div>
      <div v-else-if="itemCount === 1">
        <div class="flex">
          <span class="block w-24 truncate whitespace-nowrap text-neutral-400"
            >Relative path</span
          >
          <span class="flex-1 truncate whitespace-nowrap">
            <!-- must not be undefined since allItemsLoaded === true -->
            {{ items[0]!.item.path }}
          </span>
        </div>
        <div class="flex">
          <span class="block w-24 truncate whitespace-nowrap text-neutral-400">
            Extension
          </span>
          <span class="flex-1 truncate whitespace-nowrap">
            <!-- must not be undefined since allItemsLoaded === true -->
            {{ path.extname(items[0]!.item.path) || "(none)" }}
          </span>
        </div>
      </div>
      <div v-else>
        <div class="italic text-neutral-400">None</div>
      </div>
    </div>
  </div>
</template>
