<script lang="ts" setup>
import { ListViewColumn, state } from "@/lib/api";
import {
  createEventListenerRegistry,
  EventListenerInfo,
  findClosestIndex,
} from "@/lib/utils";
import { getSpacingSize } from "@/lib/tailwindcss";
import { computed, reactive, ref } from "vue";

const columnBreakpoints = computed(() => {
  const positions: number[] = [0];
  for (let i = 0; i < state.listViewColumns.length; i++) {
    const prevPos = positions[positions.length - 1];
    positions.push(
      prevPos + Math.max(state.listViewColumns[i].width, COLUMN_MIN_WIDTH)
    );
  }
  return positions;
});

const resizeHandleWidth = getSpacingSize("2");

const COLUMN_MIN_WIDTH = 16;

function onResizerMouseDown(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent
) {
  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  const initialWidth = col.width;
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    const newWidth = initialWidth - initialX + moveEvt.clientX;
    col.width = Math.max(Math.round(newWidth), COLUMN_MIN_WIDTH);
  });
  listeners.add(window, "mouseup", (_: MouseEvent) => {
    listeners.clear();
  });
}

const COLUMN_DRAG_THRESHOLD = 10;

function onColumnMouseDown(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent
) {
  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  listeners.add(window, "mousemove", (moveEvt: MouseEvent) => {
    const dx = moveEvt.clientX - initialX;
    if (Math.abs(dx) >= COLUMN_DRAG_THRESHOLD) {
      // remove existing listeners
      listeners.clear();
      // then let handleColumnDrag do the rest (it may create new listeners)
      handleColumnDrag(colIdx, col, downEvt, moveEvt);
    }
  });
  listeners.add(window, "mouseup", (upEvt: MouseEvent) => {
    // remove existing listeners
    listeners.clear();
    // then let handleColumnClick do the rest (it may create new listeners)
    handleColumnClick(colIdx, col, downEvt, upEvt);
  });
}

// a map from column index (a number) to offset pixels (a number)
// it's typed as a <string, number> because object keys can only be strings
const columnVisualOffsets: Record<string, number | undefined> = reactive({});

function handleColumnDrag(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent,
  moveEvt: MouseEvent
) {
  // We are working with 2 units here:
  //   client X: X position relative to the whole window
  //   offset X: X position relative to the start of the headers, which may be off-screen
  const listeners = createEventListenerRegistry();
  const initialClientX = downEvt.clientX;
  const element = downEvt.target as HTMLDivElement;
  const elementClientX = element.getBoundingClientRect().x;
  const initialOffsetX =
    initialClientX - elementClientX + columnBreakpoints.value[colIdx];
  function onMouseMove(moveEvt: MouseEvent) {
    const dx = moveEvt.clientX - initialClientX;
    columnVisualOffsets[colIdx] = dx;
  }
  onMouseMove(moveEvt);
  listeners.add(window, "mousemove", onMouseMove);
  listeners.add(window, "mouseup", (upEvt: MouseEvent) => {
    // remove existing listeners
    listeners.clear();

    // reset visual offsets
    // don't use `delete`, that messes with Vue's reactivity
    for (const key of Object.keys(columnVisualOffsets)) {
      columnVisualOffsets[key] = undefined;
    }

    // determine which index this column should be moved to
    const currentOffsetX = initialOffsetX - initialClientX + upEvt.clientX;
    const breakpointIdx = findClosestIndex(
      columnBreakpoints.value,
      currentOffsetX
    );
    const newColIdx =
      breakpointIdx > colIdx ? breakpointIdx - 1 : breakpointIdx;

    // remove the column from the list, then insert in the correct position
    if (newColIdx !== colIdx) {
      const tmp = state.listViewColumns.splice(colIdx, 1);
      state.listViewColumns.splice(newColIdx, 0, tmp[0]);
    }
  });
}

function handleColumnClick(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent,
  upEvt: MouseEvent
) {
  // TODO: Sort by this column
  console.log("YOU HAVE CLICKED THIS COLUMN!!!");
}

const debug = false;
</script>

<template>
  <div
    class="right-side-filler sticky top-0 flex h-6 w-full min-w-max border-b border-neutral-100 bg-white"
  >
    <div
      v-for="(col, i) in state.listViewColumns"
      class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
      :style="{
        width: `${Math.max(col.width, COLUMN_MIN_WIDTH)}px`,
        left: `${columnBreakpoints[i] + (columnVisualOffsets[i] || 0)}px`,
      }"
      :class="columnVisualOffsets[i] ? 'z-10 border-l opacity-50' : ''"
      @click.stop
      @mousedown="(e) => onColumnMouseDown(i, col, e)"
    >
      <template v-if="col.type === 'path'">Path</template>
      <template v-else-if="col.type === 'tags'">Tags</template>
      <template v-else-if="col.type === 'extension'">Extension</template>
      <template v-else-if="col.type === 'name'">Name</template>
      <template v-else>
        <span class="italic text-red-500">
          Not implemented, please notify the developer!
        </span>
      </template>
    </div>
    <component
      is="div"
      v-for="(col, i) in state.listViewColumns"
      class="absolute z-10 h-6 cursor-col-resize bg-red-500"
      :class="debug ? 'opacity-50' : 'opacity-0'"
      :style="{
        left: `${columnBreakpoints[i + 1] - resizeHandleWidth / 2}px`,
        width: `${resizeHandleWidth}px`,
      }"
      @mousedown="(e) => onResizerMouseDown(i, col, e)"
    />
  </div>
</template>

<style scoped>
.right-side-filler {
  box-shadow: 100vw 0 0 white;
}
</style>
