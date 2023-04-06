<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import { refreshAll } from "@/lib/api";

refreshAll();
</script>

<template>
  <div id="container" class="h-screen border border-neutral-300">
    <TitleBar />
    <!--<Main />-->
    <!--<StatusBar />-->
  </div>
</template>
