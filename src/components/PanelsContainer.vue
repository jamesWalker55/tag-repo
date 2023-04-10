<script lang="ts" setup>
import { computed, useSlots, watch } from "vue";
import type { Component } from "@vue/runtime-core";
import { PanelSizeKey, state } from "@/lib/api/state";
import BottomPanel from "@/components/BottomPanel.vue";
import LeftPanel from "@/components/LeftPanel.vue";
import RightPanel from "@/components/RightPanel.vue";
import { PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH } from "@/lib/constants";

interface Props {
  is?: string | Component;
  bottomSizeKey?: PanelSizeKey | null;
  leftSizeKey?: PanelSizeKey | null;
  rightSizeKey?: PanelSizeKey | null;
}

const props = withDefaults(defineProps<Props>(), {
  is: "div",
  bottomSizeKey: null,
  leftSizeKey: null,
  rightSizeKey: null,
});

const gridRows = computed(() => {
  const rows = ["minmax(0, 1fr)"];

  if (props.bottomSizeKey) {
    const lowerBoundedSize = Math.max(
      PANEL_MIN_HEIGHT,
      state.panelSizes[props.bottomSizeKey]
    );
    rows.push(`minmax(auto, ${lowerBoundedSize}px)`);
  }

  return rows.join(" ");
});

const gridCols = computed(() => {
  const cols = [];

  if (props.leftSizeKey) {
    const lowerBoundedSize = Math.max(
      PANEL_MIN_HEIGHT,
      state.panelSizes[props.leftSizeKey]
    );
    cols.push(`minmax(auto, ${lowerBoundedSize}px)`);
  }

  cols.push("minmax(0, 1fr)");

  if (props.rightSizeKey) {
    const lowerBoundedSize = Math.max(
      PANEL_MIN_HEIGHT,
      state.panelSizes[props.rightSizeKey]
    );
    cols.push(`minmax(auto, ${lowerBoundedSize}px)`);
  }

  return cols.join(" ");
});

const gridAreas = computed(() => {
  const areas = [];
  if (props.bottomSizeKey) {
    // 2 rows
    const row1 = [];
    const row2 = [];

    if (props.leftSizeKey) {
      row1.push("l");
      row2.push("l");
    }
    row1.push("m");
    row2.push("b");
    if (props.rightSizeKey) {
      row1.push("r");
      row2.push("b");
    }

    areas.push(row1.join(" "));
    areas.push(row2.join(" "));
  } else {
    // 1 row only
    const row = [];

    if (props.leftSizeKey) row.push("l");
    row.push("m");
    if (props.rightSizeKey) row.push("r");

    areas.push(row.join(" "));
  }

  return areas.map((row) => `"${row}"`).join(" ");
});
</script>

<template>
  <component :is="is" class="panels-grid grid">
    <slot style="grid-area: m">Main content</slot>

    <BottomPanel
      v-if="bottomSizeKey"
      :size-key="bottomSizeKey"
      style="grid-area: b"
    >
      <slot name="bottom">Bottom panel</slot>
    </BottomPanel>

    <LeftPanel
      v-if="leftSizeKey"
      :size-key="leftSizeKey"
      style="grid-area: l"
    >
      <slot name="left">Left panel</slot>
    </LeftPanel>

    <RightPanel
      v-if="rightSizeKey"
      :size-key="rightSizeKey"
      style="grid-area: r"
    >
      <slot name="right">Right panel</slot>
    </RightPanel>
  </component>
</template>

<style scoped>
.panels-grid {
  grid-template-rows: v-bind("gridRows");
  grid-template-columns: v-bind("gridCols");
  grid-template-areas: v-bind("gridAreas");
}
</style>
