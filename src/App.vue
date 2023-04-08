<script lang="ts" setup>
import TitleBar from "./components/TitleBar.vue";
import QueryBar from "./components/QueryBar.vue";
import StatusBar from "./components/StatusBar.vue";
import Item from "./components/Item.vue";
import { refreshAll, state } from "@/lib/api";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { RecycleScroller } from "vue-virtual-scroller";
import { getEmSizeInPx } from "@/lib/utils";
import { ref, Ref } from "vue";

refreshAll();

const mainElement: Ref<Element | null> = ref(null);
</script>

<template>
  <div
    id="container"
    class="relative grid h-screen max-h-screen select-none grid-rows-app border border-neutral-300 text-base"
  >
    <TitleBar class="flex-none" />
    <main
      ref="mainElement"
      class="relative grid grid-rows-[max-content_minmax(0,_1fr)]"
    >
      <QueryBar />
      <RecycleScroller
        v-if="mainElement !== null"
        class="h-full min-h-full"
        :items="state.itemIds"
        :item-size="getEmSizeInPx(mainElement) * 1.5 /* em */"
        v-slot="{ item }"
      >
        <Item :id="item" />
      </RecycleScroller>
    </main>
    <StatusBar />
  </div>
</template>
