<script lang="ts" setup>
import { removeTags } from "@/lib/api";

const props = defineProps<{ name: string; itemId: number | number[] }>();

function onClick() {
  const itemId = props.itemId;
  if (typeof itemId === "number") {
    removeTags([itemId], props.name);
  } else {
    removeTags(itemId, props.name);
  }
}
</script>

<template>
  <span
    class="cursor-pointer whitespace-nowrap hover:text-neutral-400 hover:line-through hover:decoration-slate-700 hover:decoration-1"
    @click="onClick"
  >
    <template v-if="name.indexOf(' ') !== -1">"{{ name }}"</template>
    <template v-else>{{ name }}</template>
  </span>
</template>
