<script lang="ts" setup>
defineProps<{ posX: number; posY: number }>();
</script>

<template>
  <!--suppress VueUnrecognizedDirective -->
  <table
    class="absolute rounded border border-neutral-300 bg-white text-sm drop-shadow-lg"
    :style="{ left: posX + 'px', top: posY + 'px' }"
  >
    <slot />
  </table>
</template>
