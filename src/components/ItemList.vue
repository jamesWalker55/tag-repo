<script lang="ts" setup>
import {Item} from '@/lib/items';
import {PropType} from 'vue';
import Cell from './ItemListCell.vue';
import path from 'path-browserify';

const props = defineProps({
  items: {
    type: Object as PropType<Item>,
    required: true,
  },
});


</script>

<template>
  <div>
    <div class="h-full border border-neutral-400">
      <table class="">
        <thead>
        <tr>
          <th>Name</th>
          <th>Path</th>
          <th>Size</th>
          <th>Date Modified</th>
          <th>Date Created</th>
          <th>Tags</th>
        </tr>
        </thead>
        <tbody>
        <tr v-for="item in items">
          <Cell>{{ path.basename(item.path) }}</Cell>
          <Cell>{{ item.path }}</Cell>
          <Cell>{{ item.size }}</Cell>
          <Cell>{{ item.modified }}</Cell>
          <Cell>{{ item.created }}</Cell>
          <Cell>{{ item.tags }}</Cell>
        </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
