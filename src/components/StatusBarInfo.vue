<script lang="ts" setup>
import { Feedback } from "@/lib/icons";

import { appWindow } from "@tauri-apps/api/window";
import { computed, ref } from "vue";
import * as api from "@/lib/api";
</script>

<template>
  <div class="ml-2 mr-auto text-neutral-500">
    {{ api.state.status || "No repo loaded" }}
  </div>
</template>
