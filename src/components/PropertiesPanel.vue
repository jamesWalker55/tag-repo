<script lang="ts" setup>
import { selection, state } from "@/lib/api";
import { createEventListenerRegistry } from "@/lib/utils";
import { Ref, ref } from "vue";
import PropertiesPanelEmpty from "@/components/PropertiesPanelEmpty.vue";
import PropertiesPanelSingle from "@/components/PropertiesPanelSingle.vue";
import PropertiesPanelMultiple from "@/components/PropertiesPanelMultiple.vue";

const containerElement: Ref<HTMLDivElement | null> = ref(null);

function onResizerMouseDown(downEvt: MouseEvent) {
  const listeners = createEventListenerRegistry();
  const initialY = downEvt.clientY;
  const container = containerElement.value;
  let initialHeight: number;
  if (container !== null) {
    initialHeight = container.getBoundingClientRect().height;
  } else {
    initialHeight = state.propertiesPanelHeight;
  }
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    const newHeight = initialHeight + initialY - moveEvt.clientY;
    state.propertiesPanelHeight = Math.round(newHeight);
  });
  listeners.add(window, "mouseup", (_: MouseEvent) => {
    listeners.clear();
  });
}
</script>

<template>
  <div ref="containerElement" class="flex flex-col">
    <!-- the resizer -->
    <component
      is="div"
      class="h-1.5 cursor-row-resize border-b border-t border-neutral-300 bg-neutral-50"
      @mousedown="onResizerMouseDown"
    />
    <PropertiesPanelEmpty v-if="selection.selectedCount.value === 0" />
    <PropertiesPanelSingle
      v-else-if="selection.selectedCount.value === 1"
      :id="selection.indexToItemId(selection.selected.value[0])"
    />
    <PropertiesPanelMultiple v-else />
  </div>
</template>
