<script lang="ts" setup>
import { onMounted, onUnmounted, ref } from "vue";

interface Props {
  maxDots?: number;
  refreshRate?: number;
}

const props = withDefaults(defineProps<Props>(), {
  maxDots: 3,
  refreshRate: 200,
});

const dotsCount = ref(1);

function incrementDotsCount() {
  dotsCount.value = (dotsCount.value % props.maxDots) + 1;
}

let intervalId: number;

onMounted(() => {
  intervalId = setInterval(incrementDotsCount, props.refreshRate);
});

onUnmounted(() => {
  clearInterval(intervalId);
});
</script>

<template>
  {{ ".".repeat(dotsCount) }}
</template>
