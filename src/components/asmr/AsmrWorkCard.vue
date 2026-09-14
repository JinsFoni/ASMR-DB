<script setup lang="ts">
import { computed } from "vue";
import { BookmarkCheck, CheckCircle2, ImageOff, Loader2 } from "lucide-vue-next";
import type { AsmrOnlineWork } from "../../lib/types";

const props = defineProps<{
  item: AsmrOnlineWork;
  /** 本地库中已存在的本地作品 id（用于已入库标识与跳转） */
  localWorkId?: number | null;
  /** 该卡片正在执行入库 */
  adding?: boolean;
}>();

const emit = defineEmits<{
  (e: "open", item: AsmrOnlineWork): void;
  (e: "add", item: AsmrOnlineWork): void;
}>();

const localId = computed(() => props.localWorkId ?? props.item.local_work_id ?? null);
const title = computed(() => props.item.title || props.item.rj_code || `#${props.item.id}`);

const metaText = computed(() => {
  const parts: string[] = [];
  if (props.item.dl_count) parts.push(`販売 ${props.item.dl_count.toLocaleString()}`);
  if (props.item.rating != null && props.item.rating > 0) parts.push(`★ ${props.item.rating.toFixed(1)}`);
  if (props.item.duration_sec) parts.push(`${Math.round(props.item.duration_sec / 60)}分`);
  if (parts.length) return parts.join(" · ");
  if (props.item.release) return `${props.item.release.slice(0, 10)} 发售`;
  return "";
});
</script>

<template>
  <a
    :href="localId ? `#/work/${localId}` : `#/asmr/work/${item.id}`"
    class="group block rounded-xl overflow-hidden border border-bg-border bg-bg-card hover:border-accent/50 hover:shadow-lg hover:shadow-accent/5 transition-all"
    @click.prevent="emit('open', item)"
  >
    <!-- 封面（完整不裁切） -->
    <div class="relative bg-bg-hover overflow-hidden">
      <img
        v-if="item.cover_url"
        :src="item.cover_url"
        :alt="title"
        loading="lazy"
        class="w-full h-auto group-hover:scale-105 transition-transform duration-300"
      />
      <div v-else class="w-full aspect-[4/3] flex items-center justify-center text-muted/50">
        <ImageOff :size="28" />
      </div>

      <!-- 已入库标识（本地已存在该作品） -->
      <div
        v-if="localId"
        class="absolute top-1.5 right-1.5 flex gap-1"
      >
        <span
          class="px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 backdrop-blur bg-yellow-500/90 text-white"
        >
          <BookmarkCheck :size="10" /> 已入库
        </span>
      </div>
      <!-- 本地作品（播放历史）显示已下载标识 -->
      <span
        v-else-if="localWorkId"
        class="absolute top-1.5 right-1.5 px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 backdrop-blur bg-green-500/90 text-white"
      >
        <CheckCircle2 :size="10" /> 本地
      </span>
    </div>

    <!-- 信息 -->
    <div class="p-2.5 relative">
      <div class="text-xs font-medium text-white ellipsis-2 min-h-[2.2rem]" :title="title">
        {{ title }}
      </div>
      <div class="text-[10px] text-muted mt-1 ellipsis-1">
        <span v-if="item.rj_code" class="font-mono">{{ item.rj_code }}</span>
        <span v-if="item.circle_name"> · {{ item.circle_name }}</span>
      </div>
      <div v-if="item.tags.length" class="flex flex-wrap gap-1 mt-1.5 min-h-[18px]" :class="localId || !item.rj_code ? '' : 'pr-14'">
        <span
          v-for="t in item.tags.slice(0, 3)"
          :key="t"
          class="px-1 py-0.5 rounded text-[9px] bg-bg-hover text-muted border border-bg-border"
        >
          {{ t }}
        </span>
      </div>
      <div class="flex items-center gap-2 mt-1.5 text-[10px] text-muted min-h-[14px]" :class="localId || !item.rj_code ? '' : 'pr-14'">
        <span v-if="metaText">{{ metaText }}</span>
      </div>

      <!-- 入库按钮（未入库且有 RJ 号时显示在信息区右下角） -->
      <!-- 卡片根元素是 <a>，必须 .prevent 阻止默认导航，否则点击入库会误跳详情 -->
      <button
        v-if="!localId && item.rj_code"
        class="btn-primary absolute bottom-2.5 right-2.5 !text-[10px] !px-2 !py-1"
        :disabled="adding"
        title="将此作品加入作品库"
        @click.stop.prevent="emit('add', item)"
      >
        <Loader2 v-if="adding" :size="11" class="animate-spin" />
        <BookmarkCheck v-else :size="11" />
        入库
      </button>
    </div>
  </a>
</template>
