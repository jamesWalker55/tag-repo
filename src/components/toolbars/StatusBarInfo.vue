<script lang="ts" setup>
import LoadingDots from "@/components/LoadingDots.vue";
import { ManagerStatus, state } from "@/lib/api";
</script>

<template>
  <div class="mr-auto text-neutral-500">
    <template v-if="state.status === null"> No repo loaded.</template>
    <template v-else-if="state.status === ManagerStatus.IDLE">
      Idle. {{ state.itemIds.length }} items found.
    </template>
    <template v-else-if="state.status === ManagerStatus.SCANNING_DIRECTORY">
      Scanning directory "{{ state.path }}"<LoadingDots />
    </template>
    <template v-else-if="state.status === ManagerStatus.UPDATING_REPO">
      Updating repository<LoadingDots />
    </template>
    <template v-else>
      <span class="text-red-500">
        Status {{ JSON.stringify(state.status) }} not implemented, please notify
        the developer!
      </span>
    </template>
  </div>
</template>
