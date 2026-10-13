<script setup lang="ts">
import type { WorkView } from "../../lib/types";
import WorkCard from "./WorkCard.vue";
import { useCardPlay } from "../../composables/useCardPlay";
import { useWorksStore } from "../../stores/works";

defineProps<{ works: WorkView[]; columns: number; imageHeight: number }>();
const store = useWorksStore();
const { playView, isViewCurrent, isViewLoading } = useCardPlay();
</script>

<template>
  <div
    class="grid gap-3"
    :style="{
      gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
      '--poster-img-h': `${imageHeight}px`,
    }"
  >
    <WorkCard
      v-for="w in works"
      :key="w.work.id"
      :view="w"
      :select-mode="store.selectionMode"
      :selected="store.selectedIds.has(w.work.id)"
      :play-loading="isViewLoading(w)"
      :play-current="isViewCurrent(w)"
      @toggle-select="store.toggleSelect"
      @play="playView"
    />
  </div>
</template>
