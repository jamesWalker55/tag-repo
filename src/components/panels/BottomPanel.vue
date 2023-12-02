<script lang="ts" setup>
import { state } from "@/lib/api";
import { PanelSizeKey } from "@/lib/api/state";
import { createEventListenerRegistry } from "@/lib/utils";
import { Ref, ref } from "vue";

interface Props {
  sizeKey: PanelSizeKey;
}

const props = defineProps<Props>();

const containerElement: Ref<HTMLDivElement | null> = ref(null);

function onResizerMouseDown(downEvt: MouseEvent) {
  const listeners = createEventListenerRegistry();
  const initialY = downEvt.clientY;
  const container = containerElement.value;
  let initialHeight: number;
  if (container !== null) {
    initialHeight = container.getBoundingClientRect().height;
  } else {
    initialHeight = state.panelSizes[props.sizeKey];
  }
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    const newHeight = initialHeight + initialY - moveEvt.clientY;
    state.panelSizes[props.sizeKey] = Math.round(newHeight);
  });
  listeners.add(window, "mouseup", (_: MouseEvent) => {
    listeners.clear();
  });
}
</script>

<template>
  <div ref="containerElement" class="flex flex-col overflow-clip">
    <!-- the resizer -->
    <div
      class="h-1.5 flex-none cursor-row-resize border-b border-t border-neutral-300 bg-neutral-50"
      @mousedown="onResizerMouseDown"
    />
    <div class="min-h-0 flex-1">
      <slot />
    </div>
  </div>
</template>
