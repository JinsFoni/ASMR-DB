<script setup lang="ts">
import type { WorkView } from "../../lib/types";
import WorkCard from "./WorkCard.vue";
import { useCardPlay } from "../../composables/useCardPlay";
import { useWorksStore } from "../../stores/works";

defineProps<{ works: WorkView[]; columns: number }>();
const store = useWorksStore();
const { playView, isViewCurrent, isViewLoading } = useCardPlay();
</script>

<template>
  <div class="grid gap-[22px]" :style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }">
    <WorkCard
      v-for="w in works"
      :key="w.work.id"
      :view="w"
      variant="cover"
      :select-mode="store.selectionMode"
      :selected="store.selectedIds.has(w.work.id)"
      :play-loading="isViewLoading(w)"
      :play-current="isViewCurrent(w)"
      @toggle-select="store.toggleSelect"
      @play="playView"
    />
  </div>
</template>
