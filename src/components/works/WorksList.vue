<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { CheckCircle2, ImageOff } from "lucide-vue-next";
import type { WorkView } from "../../lib/types";
import { useWorksStore } from "../../stores/works";

const props = defineProps<{ works: WorkView[] }>();
const store = useWorksStore();

const fmtDate = (d: string | null) => (d ? d.slice(0, 10) : "-");

const allSelected = computed(
  () =>
    props.works.length > 0 &&
    props.works.every((w) => store.selectedIds.has(w.work.id))
);

const statusLabel = computed(() => (s: string) => {
  switch (s) {
    case "downloaded":
      return "已下载";
    case "downloading":
      return "下载中";
    default:
      return "未下载";
  }
});
</script>

<template>
  <div class="card overflow-hidden">
    <table class="w-full text-sm">
      <thead>
        <tr class="text-left text-[11px] text-muted border-b border-bg-border">
          <th v-if="store.selectionMode" class="px-3 py-2 font-medium w-10">
            <input
              type="checkbox"
              class="accent-accent cursor-pointer"
              :checked="allSelected"
              @change="store.toggleSelectAll()"
            />
          </th>
          <th class="px-3 py-2 font-medium w-14"></th>
          <th class="px-3 py-2 font-medium">作品</th>
          <th class="px-3 py-2 font-medium">RJ号</th>
          <th class="px-3 py-2 font-medium">社团</th>
          <th class="px-3 py-2 font-medium">标签</th>
          <th class="px-3 py-2 font-medium">发售日</th>
          <th class="px-3 py-2 font-medium">状态</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="w in works"
          :key="w.work.id"
          class="border-b border-bg-border/60 last:border-0 hover:bg-bg-hover transition-colors"
        >
          <td v-if="store.selectionMode" class="px-3 py-2">
            <input
              type="checkbox"
              class="accent-accent cursor-pointer"
              :checked="store.selectedIds.has(w.work.id)"
              @change="store.toggleSelect(w.work.id)"
            />
          </td>
          <td class="px-3 py-2">
            <RouterLink :to="`/work/${w.work.id}`">
              <div class="w-10 h-13 aspect-[3/4] rounded bg-bg-hover overflow-hidden">
                <img v-if="w.work.cover_url" :src="w.work.cover_url" class="w-full h-full object-cover" loading="lazy" />
                <div v-else class="w-full h-full flex items-center justify-center text-muted/40">
                  <ImageOff :size="14" />
                </div>
              </div>
            </RouterLink>
          </td>
          <td class="px-3 py-2 max-w-[280px]">
            <RouterLink
              :to="`/work/${w.work.id}`"
              class="text-white hover:text-accent-light ellipsis-1 block"
            >
              {{ w.work.title_zh || w.work.title_ja || w.work.rj_code }}
            </RouterLink>
          </td>
          <td class="px-3 py-2 font-mono text-xs text-muted">{{ w.work.rj_code }}</td>
          <td class="px-3 py-2 text-xs text-muted ellipsis-1 max-w-[160px]">{{ w.work.circle_name || "-" }}</td>
          <td class="px-3 py-2">
            <div class="flex flex-wrap gap-1">
              <span
                v-for="t in w.tags.slice(0, 3)"
                :key="t.id"
                class="px-1 py-0.5 rounded text-[9px] border"
                :style="{ color: t.color, borderColor: t.color + '55', background: t.color + '11' }"
              >
                {{ t.name }}
              </span>
            </div>
          </td>
          <td class="px-3 py-2 text-xs text-muted">{{ fmtDate(w.work.sale_date) }}</td>
          <td class="px-3 py-2">
            <span
              class="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full"
              :class="
                w.download_status === 'downloaded'
                  ? 'bg-green-500/15 text-green-400'
                  : w.download_status === 'downloading'
                  ? 'bg-accent/15 text-accent-light'
                  : 'bg-bg-hover text-muted'
              "
            >
              <CheckCircle2 v-if="w.download_status === 'downloaded'" :size="10" />
              {{ statusLabel(w.download_status) }}
            </span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
