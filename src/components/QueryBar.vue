<script lang="ts" setup>
import { ref, watch } from "vue";

const queryText = ref("");
watch(() => queryText.value, async (text) => console.log(text));
</script>

<template>
  <input v-model="queryText" class="mx-1 px-1 py-1 text-base border border-neutral-400 outline-none" />
</template>
