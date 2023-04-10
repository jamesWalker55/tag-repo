<script lang="ts" setup>
import { FileType, ItemDetails } from "@/lib/ffi";
import { computed, ComputedRef, ref, Ref, watch } from "vue";
import { getItemDetails, selection, state } from "@/lib/api";
import ItemIcon from "@/components/itemlist/ItemIcon.vue";
import LoadingDots from "@/components/LoadingDots.vue";
import { Spinner, FTMultiple, VerticalDots, HorizontalDots } from "@/lib/icons";

enum PanelMode {
  NO_ITEMS = "NO_ITEMS",
  SINGLE_ITEM = "SINGLE_ITEM",
  MULTIPLE_ITEMS = "MULTIPLE_ITEMS",
}

const panelMode: ComputedRef<PanelMode> = computed(() => {
  if (selection.selectedCount.value === 0) {
    return PanelMode.NO_ITEMS;
  } else if (selection.selectedCount.value === 1) {
    return PanelMode.SINGLE_ITEM;
  } else {
    return PanelMode.MULTIPLE_ITEMS;
  }
});

const items: Ref<ItemDetails[] | null> = ref(null);
{
  let latestPromise = null;

  async function fetchItems(selectedIndexes: number[]) {
    console.log("fetching items:", selectedIndexes);

    // set to `null` to indicate properties is loading
    items.value = null;

    const promises = [];
    for (const index of selectedIndexes) {
      const itemId = selection.indexToItemId(index);
      promises.push(getItemDetails(itemId));
    }
    // const promises = selectedIndexes.map((index) => {
    //   const itemId = selection.indexToItemId(index);
    //   return getItemDetails(itemId);
    // });
    const allPromises = Promise.all(promises);
    latestPromise = allPromises;
    const newItems = await Promise.all(promises);
    if (latestPromise !== allPromises) {
      // another promise was requested while we were awaiting
      // discard this promise
      return;
    }
    items.value = newItems;
    // items.value = null;
  }

  watch(
    selection.selected,
    fetchItems,
    // `deep` is necessary to monitor the selection correctly.
    // Why? The selection is sometimes changed by pushing to its array, but
    // sometimes changed by directly replacing with a new selection object.
    // Without `deep`, the watch function can only monitor when the selection
    // object gets replaced. If an item is pushed / spliced to the selection,
    // that will not be detected by the watch unless `deep` is true.
    { deep: true }
  );

  // fetch data asynchronously
  fetchItems(selection.selected.value).then();
}
</script>

<template>
  <div class="flex flex-col h-full px-3 py-2">
    <!-- title bar -->
    <div class="mb-5 flex h-5 flex-none flex-row items-center gap-2">
      <!-- icon -->
      <ItemIcon
        v-if="items === null"
        :filetype="FileType.UNKNOWN"
        class="h-16px w-16px flex-none animate-pulse rounded-full text-neutral-400"
      />
      <ItemIcon
        v-else-if="items.length === 0"
        :filetype="FileType.UNKNOWN"
        class="h-16px w-16px flex-none text-neutral-500"
      />
      <ItemIcon
        v-else-if="items.length === 1"
        :filetype="items[0].filetype"
        class="h-16px w-16px flex-none text-neutral-600"
      />
      <FTMultiple v-else class="flex-0 h-16px w-16px text-neutral-600" />
      <!-- text -->
      <span
        is="div"
        v-if="items === null"
        class="h-4 flex-1 animate-pulse rounded-full bg-neutral-100 italic"
      />
      <span
        v-else-if="items.length === 0"
        class="min-w-0 flex-1 truncate whitespace-nowrap italic text-neutral-400"
      >
        No item selected
      </span>
      <span
        v-else-if="items.length === 1"
        class="min-w-0 flex-1 truncate whitespace-nowrap"
      >
        {{ items[0].item.path }}
      </span>
      <span v-else class="min-w-0 flex-1 truncate whitespace-nowrap">
        Multiple items
      </span>
      <!-- button -->
      <VerticalDots class="h-16px w-16px flex-none" />
    </div>
    <!-- tags list -->
    <div class="flex flex-1 flex-col">
      <div class="font-bold text-neutral-500">Tags</div>
      <div v-if="items === null" class="animate-pulse italic text-neutral-400">
        Loading<LoadingDots />
      </div>
      <div v-else-if="items.length === 0" class="italic text-neutral-400">
        No tags
      </div>
      <div v-else-if="items.length === 1" class="">
        a apple b bee cat dog egg tag test adipiscing aliquam amet arcu consectetur dolor dui elit est etiam id imperdiet in ipsum lacus libero ligula lorem massa molestie pellentesque quis risus sagittis sit suscipit suspendisse ullamcorper vel vestibulum
      </div>
      <!-- TODO: You must represent tags as a list in the backend! -->
    </div>
    <div class="flex-none">
      <div class="font-bold text-neutral-600">Properties</div>
      <div v-if="items === null">
        <div></div>
        <div>{{items}}</div>
      </div>
      <div v-else-if="items.length === 0">
        <div></div>
        <div>{{items}}</div>
      </div>
      <div v-else-if="items.length === 1">
        <div class="flex">
          <span class="block w-2">Full path:</span>
          <span>Full path:</span>
        </div>
        <div>{{items}}</div>
      </div>
      <div v-else>
        <div></div>
        <div>{{items}}</div>
      </div>
    </div>
    <!--<div class="h-36"></div>-->
    <!--<div><b>Panel mode:</b> {{ panelMode }}</div>-->
    <!--<div><b>Selection:</b> {{ selection.selected }}</div>-->
    <!--<div><b>"items" array:</b> {{ items?.map((x) => x.item.path) }}</div>-->
    <!--{{-->
    <!--  selection.selected.value.map((index) => {-->
    <!--    const itemId = selection.indexToItemId(index);-->
    <!--    // return getCachedItem(itemId);-->
    <!--  })-->
  </div>
</template>

<style scoped></style>
