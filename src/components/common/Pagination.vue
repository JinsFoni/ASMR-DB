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
  <div class="flex items-center justify-center gap-3 pt-8">
    <button class="pager-btn" :disabled="page <= 1" @click="go(page - 1)">
      <ChevronLeft :size="14" /> 上一页
    </button>
    <template v-for="(p, i) in pages" :key="i">
      <button v-if="p === '...'" class="px-1 text-xs" style="color: var(--label-3)" disabled>…</button>
      <button v-else class="pager-btn num" :class="{ on: p === page }" @click="go(p)">
        {{ p }}
      </button>
    </template>
    <button class="pager-btn" :disabled="page >= total" @click="go(page + 1)">
      下一页 <ChevronRight :size="14" />
    </button>
  </div>
</template>

<style scoped>
.pager-btn {
  height: 34px;
  padding: 0 16px;
  border: none;
  border-radius: 999px;
  background: var(--fill-1);
  color: var(--label-2);
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  transition: background 0.2s, color 0.2s;
}
.pager-btn:hover:not(:disabled) {
  background: var(--fill-2);
  color: var(--label);
}
.pager-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.pager-btn.num {
  width: 34px;
  padding: 0;
  justify-content: center;
}
.pager-btn.on {
  background: var(--blue);
  color: #fff;
}
</style>
