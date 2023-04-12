<script lang="ts" setup>
import { Component as ComponentType } from "@vue/runtime-core";
import { MenuMore } from "@/lib/icons";

interface Props {
  text: string;
  altText?: string;
  disabled?: boolean;
}
defineProps<Props>();
</script>

<template>
  <tr class="group">
    <td class="pl-3 group-hover:bg-neutral-100">
      <slot
        name="icon"
        v-bind="{ defaultClasses: 'text-base text-neutral-600' }"
      ></slot>
    </td>
    <td class="flex h-8 items-center px-3 group-hover:bg-neutral-100">
      {{ text }}
    </td>
    <td class="pr-3 text-right group-hover:bg-neutral-100">{{ altText }}</td>
    <td v-if="$slots.default" class="pr-3 group-hover:bg-neutral-100">
      <MenuMore />
      <!-- temporary, implement this when you have time -->
      <slot />
    </td>
    <td v-else class="pr-3 group-hover:bg-neutral-100"></td>
  </tr>
</template>
