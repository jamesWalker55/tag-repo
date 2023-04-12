<script lang="ts" setup>
defineProps<{ posX: number; posY: number }>();
</script>

<template>
  <!--suppress VueUnrecognizedDirective -->
  <div
    class="menu-grid absolute flex grid flex-col items-center justify-items-stretch rounded border border-neutral-300 bg-white text-sm drop-shadow-lg"
    :style="{ left: posX + 'px', top: posY + 'px' }"
  >
    <slot />
  </div>
</template>

<style scoped>
.menu-grid {
  grid-template-columns: max-content max-content max-content max-content;
}
</style>
