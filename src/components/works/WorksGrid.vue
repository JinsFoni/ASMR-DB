<script setup lang="ts">
import type { WorkView } from "../../lib/types";
import WorkCard from "./WorkCard.vue";
import { useWorksStore } from "../../stores/works";

defineProps<{ works: WorkView[] }>();
const store = useWorksStore();
</script>

<template>
  <div class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3">
    <WorkCard
      v-for="w in works"
      :key="w.work.id"
      :view="w"
      :select-mode="store.selectionMode"
      :selected="store.selectedIds.has(w.work.id)"
      @toggle-select="store.toggleSelect"
    />
  </div>
</template>
