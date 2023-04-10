<script lang="ts" setup>
import { state } from "@/lib/api";
import { createEventListenerRegistry } from "@/lib/utils";
import { Ref, ref } from "vue";
import { PanelSizeKey } from "@/lib/api/state";

interface Props {
  sizeKey: PanelSizeKey;
}

const props = defineProps<Props>();

const containerElement: Ref<HTMLDivElement | null> = ref(null);

function onResizerMouseDown(downEvt: MouseEvent) {
  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  const container = containerElement.value;
  let initialWidth: number;
  if (container !== null) {
    initialWidth = container.getBoundingClientRect().width;
  } else {
    initialWidth = state.panelSizes[props.sizeKey];
  }
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    const newWidth = initialWidth + initialX - moveEvt.clientX;
    state.panelSizes[props.sizeKey] = Math.round(newWidth);
  });
  listeners.add(window, "mouseup", (_: MouseEvent) => {
    listeners.clear();
  });
}
</script>

<template>
  <div ref="containerElement" class="flex flex-row overflow-clip">
    <!-- the resizer -->
    <component
      is="div"
      class="w-1.5 flex-none cursor-col-resize border-l border-r border-neutral-300 bg-neutral-50"
      @mousedown="onResizerMouseDown"
    />
    <div class="min-w-0 flex-1">
      <slot />
    </div>
  </div>
</template>
