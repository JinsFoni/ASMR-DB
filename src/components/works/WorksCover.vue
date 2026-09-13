<script setup lang="ts">
import type { WorkView } from "../../lib/types";
import WorkCard from "./WorkCard.vue";
import { useWorksStore } from "../../stores/works";

defineProps<{ works: WorkView[]; columns: number }>();
const store = useWorksStore();
</script>

<template>
  <div class="grid gap-3" :style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }">
    <WorkCard
      v-for="w in works"
      :key="w.work.id"
      :view="w"
      variant="cover"
      :select-mode="store.selectionMode"
      :selected="store.selectedIds.has(w.work.id)"
      @toggle-select="store.toggleSelect"
    />
  </div>
</template>
