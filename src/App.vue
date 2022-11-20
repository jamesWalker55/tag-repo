<script lang="ts" setup>
import TitleBar from './components/TitleBar.vue';
import SearchBar from './components/SearchBar.vue';
import ItemList from './components/ItemList.vue';
import {Item} from '@/lib/items';
import {computed, ComputedRef} from 'vue';

const items: ComputedRef<Item[]> = computed(() => [
  {
    path: 'apple/iphone.txt',
    size: 4241280000,
  }
]);
</script>

<template>
  <div id="container" class="flex flex-col h-screen border border-neutral-300">
    <TitleBar class="flex-none"/>
    <SearchBar class="flex-none px-1 py-0.5"/>
    <ItemList class="flex-1 p-1 pt-0.5" :items="items"/>
    <!--<StatusBar/>-->
  </div>
</template>

