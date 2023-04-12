<script lang="ts" setup>
import type { Component } from "@vue/runtime-core";
import { MenuMore } from "@/lib/icons";

interface Props {
  text: string;
  altText?: string;
  icon?: string | Component;
  disabled?: boolean;
}
defineProps<Props>();
</script>

<template>
  <component v-if="icon" :is="icon" class="ml-3 text-base text-neutral-600" />
  <div v-else></div>
  <div class="mx-3 flex h-8 items-center">{{ text }}</div>
  <div class="ml-2 mr-3 text-right">{{ altText }}</div>
  <div v-if="$slots.default" class="mr-3"><MenuMore /></div>
  <div v-else></div>
</template>
