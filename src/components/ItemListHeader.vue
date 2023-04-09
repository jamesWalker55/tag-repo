<script lang="ts" setup>
import { ListViewColumn, state } from "@/lib/api";
import { createEventListenerRegistry, EventListenerInfo } from "@/lib/utils";
import { getSpacingSize } from "@/lib/tailwindcss";
import { computed } from "vue";

const columnBreakpoints = computed(() => {
  const positions: number[] = [0];
  for (let i = 0; i < state.listViewColumns.length; i++) {
    const prevPos = positions[positions.length - 1];
    positions.push(prevPos + state.listViewColumns[i].width);
  }
  return positions;
});

const resizeHandleWidth = getSpacingSize("2");

const log = console.log;

const windowHandle = window;

function onResizerMouseDown(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent
) {
  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  const initialWidth = col.width;
  listeners.add(windowHandle, "mousemove", (moveEvt: MouseEvent) => {
    const newWidth = initialWidth - initialX + moveEvt.clientX;
    col.width = Math.round(newWidth);
  });
  listeners.add(windowHandle, "mouseup", (_: MouseEvent) => {
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
  listeners.add(windowHandle, "mousemove", (moveEvt: MouseEvent) => {
    const dx = moveEvt.clientX - initialX;
    if (Math.abs(dx) >= COLUMN_DRAG_THRESHOLD) {
      // remove existing listeners
      listeners.clear();
      // then let handleColumnDrag do the rest (it may create new listeners)
      handleColumnDrag(colIdx, col, downEvt, moveEvt);
    }
  });
  listeners.add(windowHandle, "mouseup", (upEvt: MouseEvent) => {
    // remove existing listeners
    listeners.clear();
    // then let handleColumnClick do the rest (it may create new listeners)
    handleColumnClick(colIdx, col, downEvt, upEvt);
  });
}

function handleColumnDrag(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent,
  moveEvt: MouseEvent
) {
  const listeners = createEventListenerRegistry();
  const initialX = downEvt.clientX;
  function onMouseMove(moveEvt: MouseEvent) {
    const dx = moveEvt.clientX - initialX;
    // TODO: SOME CODE TO VISUALLY MOVE THE HEADER
  }
  onMouseMove(moveEvt);
  listeners.add(windowHandle, "mousemove", onMouseMove);
  listeners.add(windowHandle, "mouseup", (upEvt: MouseEvent) => {
    // remove existing listeners
    listeners.clear();
    // // commit changes by editing the headers list
    // state.listViewColumns
  });
}

function handleColumnClick(
  colIdx: number,
  col: ListViewColumn,
  downEvt: MouseEvent,
  upEvt: MouseEvent
) {
  // TODO: Sort by this column
}

const debug = false;
</script>

<template>
  <div
    class="sticky top-0 flex h-6 w-full min-w-max border-b border-neutral-300 bg-neutral-100"
  >
    <template v-for="(col, i) in state.listViewColumns">
      <div
        v-if="col.type === 'path'"
        class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px`, left: `${columnBreakpoints[i]}px` }"
        @mousedown="(e) => onColumnMouseDown(i, col, e)"
      >
        Path
      </div>
      <div
        v-else-if="col.type === 'tags'"
        class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px`, left: `${columnBreakpoints[i]}px` }"
        @mousedown="(e) => onColumnMouseDown(i, col, e)"
      >
        Tags
      </div>
      <div
        v-else-if="col.type === 'extension'"
        class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px`, left: `${columnBreakpoints[i]}px` }"
        @mousedown="(e) => onColumnMouseDown(i, col, e)"
      >
        Extension
      </div>
      <div
        v-else-if="col.type === 'name'"
        class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 hover:bg-slate-100"
        :style="{ width: `${col.width}px`, left: `${columnBreakpoints[i]}px` }"
        @mousedown="(e) => onColumnMouseDown(i, col, e)"
      >
        Name
      </div>
      <div
        v-else
        class="absolute flex h-6 flex-none items-center border-b border-r border-neutral-300 bg-white px-2 italic text-red-500 hover:bg-slate-100"
        :style="{ width: `${col.width}px`, left: `${columnBreakpoints[i]}px` }"
        @mousedown="(e) => onColumnMouseDown(i, col, e)"
      >
        Not implemented, please notify the developer!
      </div>
    </template>
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
