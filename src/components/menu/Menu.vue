<script lang="ts" setup>
import { Ref, ref, watch } from "vue";

defineProps<{ posX: number; posY: number }>();

// emit "resized" event when we finally get the size
interface Emits {
  (e: "resized", width: number, height: number): void;
}

const emit = defineEmits<Emits>();

// the element
const el: Ref<Element | null> = ref(null);

// update the dimension refs when menu has been rendered
watch(el, (newEl) => {
  if (newEl === null) return;

  const clientRect = newEl.getBoundingClientRect();
  emit("resized", clientRect.width, clientRect.height);
});
</script>

<template>
  <!--suppress VueUnrecognizedDirective -->
  <table
    ref="el"
    class="absolute rounded border border-neutral-300 bg-white text-sm drop-shadow-lg"
    :style="{ left: posX + 'px', top: posY + 'px' }"
  >
    <slot />
  </table>
</template>
