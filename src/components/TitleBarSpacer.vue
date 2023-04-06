<script lang="ts" setup>
import * as api from "@/lib/api";
</script>

<template>
  <div class="mx-auto truncate" data-tauri-drag-region>
    {{ api.state.path }}
  </div>
</template>

<style scoped></style>
