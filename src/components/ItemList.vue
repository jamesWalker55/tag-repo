<script lang="ts" setup>
import { RecycleScroller } from "vue-virtual-scroller";
import ItemListHeader from "@/components/ItemListHeader.vue";
import ItemRow, { Column } from "./ItemRow.vue";
import { state } from "@/lib/api";
import { parseRemSize } from "@/lib/utils";
import tailwind, { getSpacingSize } from "@/lib/tailwindcss";
import { computed } from "@vue/reactivity";
import { onBeforeUnmount, onMounted, onUnmounted, Ref, ref, watch } from "vue";

const container: Ref<HTMLDivElement | null> = ref(null);

type EventListenerInfo = [
  element: Element,
  type: string,
  listener: (this: Element, ev: Event) => any
];

const eventListeners: EventListenerInfo[] = [];

function registerEventListener(
  element: Element,
  type: string,
  listener: (this: Element, ev: Event) => any
) {
  element.addEventListener(type, listener);
  eventListeners.push([element, type, listener]);
}

function clearEventListeners() {
  for (const [element, type, listener] of eventListeners) {
    element.removeEventListener(type, listener);
  }
  // setting length to 0 clears the array
  eventListeners.length = 0;
}

let observer: ResizeObserver | null = null;

const viewWidth: Ref<number> = ref(0);
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
  registerEventListener(con, "scroll", (evt: Event) => {
    updateScrollPosition(con);
  });
});

onBeforeUnmount(() => {
  // component hasn't been mounted yet, container MUST be a div at this point
  const con = container.value!;
  clearEventListeners();
  observer?.disconnect();
});

// set item height to Tailwind's 'h-6'
// keep this in sync with ItemRow's height
const itemHeight = getSpacingSize("6");
const headerHeight = getSpacingSize("6");

const containerHeight = computed(() => state.itemIds.length * itemHeight);
const containerWidth = computed(() =>
  state.listViewColumns.reduce((acc, col) => acc + col.width, 0)
);

const preloadPadding = itemHeight * 10; // px

const indexRangeToRender = computed(() => {
  const renderTop = scrollTop.value - preloadPadding;
  const renderBottom = scrollTop.value + viewHeight.value + preloadPadding;
  const itemsBeforeTop = Math.floor(renderTop / itemHeight);
  const itemsUntilBottom = Math.ceil(renderBottom / itemHeight);
  let startIndex = Math.max(itemsBeforeTop - 1, 0);
  // don't subtract 1 here, because a for-loop ends before the last value
  let endIndex = Math.min(itemsUntilBottom, state.itemIds.length);
  return [startIndex, endIndex];
});

const debug = false;
</script>

<template>
  <div ref="container" class="relative h-full w-full overflow-auto text-sm">
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
      class="absolute"
      :style="{
        top: `${(n + indexRangeToRender[0] - 1) * itemHeight + headerHeight}px`,
      }"
      :key="n + indexRangeToRender[0] - 1"
    />
    <div class="fixed bottom-2 right-2 border bg-white opacity-50 shadow" v-if="debug">
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
  </div>
</template>
