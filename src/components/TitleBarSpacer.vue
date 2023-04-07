<script lang="ts" setup>
import * as api from "@/lib/api";
</script>

<template>
  <div
    class="mx-auto flex items-center truncate text-xs text-neutral-400"
    data-tauri-drag-region
  >
    {{ api.state.path ? api.state.path + " - " : "" }}tagrepo
  </div>
</template>

<style scoped></style>
