<script lang="ts" setup>
import { Ref, ref, watch } from "vue";

const inputElement: Ref<HTMLInputElement | null> = ref(null);
const queryText = ref("");
// the '!' operator asserts that the object must be non-null
const canvasContext = document.createElement("canvas").getContext("2d")!;

function getTextWidth(text: string, font: string) {
  canvasContext.font = font;
  const metrics = canvasContext.measureText(text);
  return metrics.width;
}

function getFontStyle(element: Element) {
  const style = window.getComputedStyle(element);
  return `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
}

/**
 * Compute the width of the input text query, in pixels.
 * i.e. If you create an absolute <div> with position "left: getInputTextWidth() px", the div will
 * follow the end of the text.
 */
function getInputTextWidth() {
  const el = inputElement.value;
  if (el === null)
    throw "Failed to get input text width, element not initialised.";

  const style = getFontStyle(el);
  const width = getTextWidth(queryText.value, style);

  return width;
}

// time-based callbacks
(function () {
  let searchTimerId: number | null = null;
  watch(queryText, async (text) => {
    if (searchTimerId !== null) {
      clearTimeout(searchTimerId);
    }
    searchTimerId = setTimeout(() => {
      searchTimerId = null;
      console.log("Execute search with:", text);
    }, 300);
  });
})();
</script>

<template>
  <input
    v-model="queryText"
    spellcheck="false"
    ref="inputElement"
    class="mx-1 my-1 border border-neutral-400 px-1 py-1 text-base outline-none focus:border-neutral-600 focus:shadow-sm"
  />
</template>
