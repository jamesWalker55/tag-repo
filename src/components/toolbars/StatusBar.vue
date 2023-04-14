<script lang="ts" setup>
import FeedbackModal from "@/components/FeedbackModal.vue";
import { ref } from "vue";
import { ManagerStatus, state } from "@/lib/api";
import LoadingDots from "@/components/LoadingDots.vue";
import ToolbarButton from "@/components/toolbars/ToolbarButton.vue";

const feedbackPopup = ref(false);
</script>

<template>
  <div
    class="flex h-5 min-w-0 flex-row items-center border-x-0 border-b-0 border-t border-solid border-neutral-200 bg-neutral-50 px-2 text-sm"
  >
    <div
      class="mr-auto min-w-0 overflow-clip whitespace-nowrap text-neutral-500"
    >
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
          Status {{ JSON.stringify(state.status) }} not implemented, please
          notify the developer!
        </span>
      </template>
    </div>
    <ToolbarButton @click="() => (feedbackPopup = !feedbackPopup)">
      <i-fluent-person-feedback-16-regular width="16" height="16" />
    </ToolbarButton>
  </div>
  <FeedbackModal
    v-if="feedbackPopup"
    @closed="feedbackPopup = !feedbackPopup"
  />
</template>
