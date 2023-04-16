<script lang="ts" setup>
import ItemListHeader from "@/components/itemlist/ItemListHeader.vue";
import ItemRow from "./itemlist/ItemRow.vue";
import { actions, selection, state } from "@/lib/api";
import { createEventListenerRegistry } from "@/lib/utils";
import { getSpacingSize } from "@/lib/tailwindcss";
import { computed, onBeforeUnmount, onMounted, ref, Ref } from "vue";
import ContextMenu from "@/components/ContextMenu.vue";
import { CopyFilePath, OpenFile, RevealFile } from "@/lib/icons";
import MenuItem from "@/components/menu/MenuItem.vue";
import MenuSeparator from "@/components/menu/MenuSeparator.vue";
import { launchSelectedItems } from "@/lib/api/actions";

const container: Ref<HTMLDivElement | null> = ref(null);

const listeners = createEventListenerRegistry();
let observer: ResizeObserver | null = null;

// width of the viewport
const viewWidth: Ref<number> = ref(0);
// height of the viewport
// NOTE: this includes the header as well.
// to exclude the header, use virtualViewHeight
const viewHeight: Ref<number> = ref(0);

function updateViewSize(container: HTMLDivElement) {
  viewWidth.value = Math.round(container.clientWidth);
  viewHeight.value = Math.round(container.clientHeight);
}

const scrollTop: Ref<number> = ref(0);
const scrollLeft: Ref<number> = ref(0);

function updateScrollPosition(container: HTMLDivElement) {
  scrollTop.value = Math.round(container.scrollTop);
  scrollLeft.value = Math.round(container.scrollLeft);
}

onMounted(() => {
  // component is mounted, container MUST be a div at this point
  const con = container.value!;

  // Detect viewport size of the container
  updateViewSize(con);
  observer = new ResizeObserver((entries) => {
    // the only purpose of this section is to check if the container has been resized
    let containerWasResized = false;
    for (const entry of entries) {
      if (entry.target !== con) {
        console.warn("got entry for wrong target!", entry.target);
        continue;
      }
      if (entry.borderBoxSize.length !== 1) {
        console.warn(
          "expected only 1 size, but got not a 1!",
          entry.borderBoxSize.length
        );
        continue;
      }
      containerWasResized = true;
      break;
    }
    // if it's been resized, update the refs
    if (containerWasResized) {
      updateViewSize(con);
    }
  });
  observer.observe(con);

  // Detect scroll position within the container
  updateScrollPosition(con);
  listeners.add(con, "scroll", (evt: Event) => {
    updateScrollPosition(con);
  });
});

onBeforeUnmount(() => {
  // component hasn't been mounted yet, container MUST be a div at this point
  listeners.clear();
  observer?.disconnect();
});

// set item height to Tailwind's 'h-6'
// keep this in sync with ItemRow's height
const itemHeight = getSpacingSize("6");
const headerHeight = getSpacingSize("6");

// the actual amount of vertical space that's rendering items
// you need to subtract the header
const virtualViewHeight = computed(() => viewHeight.value - headerHeight);

const containerHeight = computed(
  () => headerHeight + state.itemIds.length * itemHeight
);
const containerWidth = computed(() =>
  state.listViewColumns.reduce((acc, col) => acc + col.width, 0)
);

const preloadPadding = itemHeight * 10; // px

const indexRangeToRender = computed(() => {
  const renderTop = scrollTop.value - preloadPadding;
  const renderBottom =
    scrollTop.value + virtualViewHeight.value + preloadPadding;
  const itemsBeforeTop = Math.floor(renderTop / itemHeight);
  const itemsUntilBottom = Math.ceil(renderBottom / itemHeight);

  // fix - if the view is out of bounds (way below the list of items)
  // a bug occurs when you execute a new query while scrolled down, and the new list is shorter than the previous
  // you need to limit BOTH the start and end index with BOTH max and min values
  const actualItemsCount = state.itemIds.length;
  // subtract 1 here, we're now returning indexes that start from 0, so it's (item count - 1)
  const startIndex = Math.max(0, Math.min(itemsBeforeTop - 1, actualItemsCount));
  // don't subtract 1 here, because a for-loop ends before the last value
  const endIndex = Math.max(0, Math.min(itemsUntilBottom, actualItemsCount));

  return [startIndex, endIndex];
});

const debug = false;

const menu = ref<InstanceType<typeof ContextMenu> | null>(null);

function scrollToIndex(index: number) {
  const el = container.value;
  if (el === null) return;

  const viewTop = el.scrollTop;
  const viewBottom = el.scrollTop + virtualViewHeight.value;

  const itemTop = index * itemHeight;
  const itemBottom = (index + 1) * itemHeight;

  if (itemTop < viewTop) {
    // item is above the view
    el.scrollTop = itemTop;
  } else if (itemBottom > viewBottom) {
    // item is below the view
    el.scrollTop = itemBottom - virtualViewHeight.value;
  } else {
    // item is in view, do nothing
  }
}

function scrollToFocusedIndex() {
  const focusedIndex = selection.focusedIndex();
  if (focusedIndex !== null) {
    scrollToIndex(focusedIndex);
  }
}

defineExpose({
  focus: () => {
    if (state.itemIds.length > 0) {
      selection.isolate(0);
      scrollToFocusedIndex();
    }
    container.value?.focus();
  },
});

const log = console.log;
</script>

<template>
  <div
    ref="container"
    class="relative h-full w-full overflow-auto border-r-2 border-white text-sm focus:outline-none"
    @click="
      (e) => {
        // This is disabled for now, due to a bug.
        // If you drag your mouse across multiple rows, it gets treated as a click on
        // this element.
        // selection.clear();
      }
    "
    @contextmenu.prevent.stop="(e) => menu?.show(e)"
    tabindex="-1"
    @keydown.enter="launchSelectedItems"
    @keydown.up.prevent="
      (e) => {
        if (e.shiftKey) {
          selection.extendUp();
        } else {
          selection.isolateUp();
        }
        scrollToFocusedIndex();
      }
    "
    @keydown.down.prevent="
      (e) => {
        if (e.shiftKey) {
          selection.extendDown();
        } else {
          selection.isolateDown();
        }
        scrollToFocusedIndex();
      }
    "
    @keydown="
      (e) => {
        // don't do anything if there's nothing in the list
        if (state.itemIds.length === 0) return;

        if (e.key === 'Home') {
          selection.isolate(0);
          scrollToFocusedIndex();
        } else if (e.key === 'End') {
          selection.isolate(state.itemIds.length - 1);
          scrollToFocusedIndex();
        } else if (e.key === 'a') {
          if (e.ctrlKey) {
            selection.selectAll();
          }
        } else {
          // event is not handled, return early
          return;
        }

        e.preventDefault();
      }
    "
  >
    <!-- The container resizer, it's a 1px div located at the bottom right corner -->
    <component
      is="div"
      class="absolute -z-10 h-px w-px bg-red-500 opacity-0"
      :style="{
        top: containerHeight - 1 + 'px',
        left: containerWidth - 1 + 'px',
      }"
    />
    <ItemRow
      v-for="n in indexRangeToRender[1] - indexRangeToRender[0]"
      :id="state.itemIds[n + indexRangeToRender[0] - 1]"
      :listIndex="n + indexRangeToRender[0] - 1"
      class="absolute"
      :style="{
        top: `${(n + indexRangeToRender[0] - 1) * itemHeight + headerHeight}px`,
      }"
      :key="state.itemIds[n + indexRangeToRender[0] - 1]"
    />
    <div
      class="fixed bottom-2 right-2 border bg-white opacity-50 drop-shadow"
      v-if="debug"
    >
      {{ indexRangeToRender }}
      <template
        class="mr-1"
        v-for="n in indexRangeToRender[1] - indexRangeToRender[0]"
      >
        {{ n + indexRangeToRender[0] - 1 }}{{ " " }}
      </template>
    </div>
    <!-- I put the header after the items to make it appear above the items -->
    <ItemListHeader />
    <ContextMenu ref="menu" v-slot="{ closeMenu }">
      <MenuItem
        :text="
          selection.selectedCount.value === 1 ? 'Open' : 'Open selected files'
        "
        @click="
          (e) => {
            actions.launchSelectedItems();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <OpenFile class="h-16px w-16px" :class="defaultClasses" />
        </template>
      </MenuItem>
      <MenuItem
        :text="
          selection.selectedCount.value === 1
            ? 'Reveal in folder'
            : 'Reveal files in folder'
        "
        @click="
          (e) => {
            actions.revealSelectedItems();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <RevealFile class="h-16px w-16px" :class="defaultClasses" />
        </template>
      </MenuItem>
      <MenuSeparator />
      <MenuItem
        :text="selection.selectedCount.value === 1 ? 'Copy path' : 'Copy paths'"
        @click="
          (e) => {
            actions.copySelectedItemPaths();
            closeMenu();
          }
        "
      >
        <template #icon="{ defaultClasses }">
          <CopyFilePath class="h-16px w-16px" :class="defaultClasses" />
        </template>
      </MenuItem>
    </ContextMenu>
  </div>
</template>
