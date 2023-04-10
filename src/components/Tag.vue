<script lang="ts" setup>
const props = defineProps<{ name: string; itemId: number | number[] }>();

function onClick() {
  // TODO: Remove this tag from the item
  console.log("Removing tag", props.name, "from items", props.itemId);
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
