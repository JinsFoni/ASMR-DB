<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, ChevronRight } from "lucide-vue-next";

const props = defineProps<{ page: number; total: number }>();
const emit = defineEmits<{ change: [page: number] }>();

const pages = computed(() => {
  const arr: (number | "...")[] = [];
  const total = props.total;
  if (total <= 7) {
    for (let i = 1; i <= total; i++) arr.push(i);
  } else {
    arr.push(1);
    if (props.page > 3) arr.push("...");
    for (let i = Math.max(2, props.page - 1); i <= Math.min(total - 1, props.page + 1); i++) arr.push(i);
    if (props.page < total - 2) arr.push("...");
    arr.push(total);
  }
  return arr;
});

function go(p: number) {
  if (p >= 1 && p <= props.total) emit("change", p);
}
</script>

<template>
  <div class="flex items-center justify-center gap-1 py-4">
    <button class="btn-ghost !p-1.5" :disabled="page <= 1" @click="go(page - 1)">
      <ChevronLeft :size="15" />
    </button>
    <template v-for="(p, i) in pages" :key="i">
      <button
        v-if="p === '...'"
        class="px-2 text-muted text-xs"
        disabled
      >...</button>
      <button
        v-else
        class="min-w-7 h-7 px-2 rounded-lg text-xs transition-colors"
        :class="p === page ? 'bg-accent text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
        @click="go(p)"
      >
        {{ p }}
      </button>
    </template>
    <button class="btn-ghost !p-1.5" :disabled="page >= total" @click="go(page + 1)">
      <ChevronRight :size="15" />
    </button>
  </div>
</template>
