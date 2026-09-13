<script setup lang="ts">
import { computed } from "vue";
import { CheckCircle2, BookmarkCheck, ImageOff, Loader2 } from "lucide-vue-next";
import type { DlsiteRankingItem } from "../../lib/types";

const props = defineProps<{
  item: DlsiteRankingItem;
  /** 该卡片正在执行入库 */
  adding?: boolean;
}>();

const emit = defineEmits<{
  (e: "open", item: DlsiteRankingItem): void;
  (e: "add", item: DlsiteRankingItem): void;
}>();

const inLibrary = computed(() => props.item.work_id != null);

/** 排名角标配色：前三名金/银/铜，其余深色 */
const rankMeta = computed(() => {
  switch (props.item.rank) {
    case 1:
      return "bg-yellow-400/90 text-black";
    case 2:
      return "bg-gray-300/90 text-black";
    case 3:
      return "bg-amber-600/90 text-white";
    default:
      return "bg-black/60 text-white";
  }
});

const title = computed(() => props.item.title || props.item.rj_code);
</script>

<template>
  <a
    :href="inLibrary ? `#/work/${item.work_id}` : `#/dlsite/work/${item.rj_code}`"
    class="group block rounded-xl overflow-hidden border border-bg-border bg-bg-card hover:border-accent/50 hover:shadow-lg hover:shadow-accent/5 transition-all"
    @click.prevent="emit('open', item)"
  >
    <!-- 封面（横版，完整不裁切，与作品库横版卡片一致） -->
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

      <!-- 排名角标 -->
      <div
        class="absolute top-1.5 left-1.5 px-1.5 py-0.5 rounded text-[10px] font-bold backdrop-blur"
        :class="rankMeta"
        :title="`排行榜第 ${item.rank} 名`"
      >
        {{ item.rank }}
      </div>

      <!-- 已入库 / 已下载角标 -->
      <div v-if="inLibrary" class="absolute top-1.5 right-1.5 flex gap-1">
        <span
          v-if="item.download_status === 'downloaded'"
          class="px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 backdrop-blur bg-green-500/90 text-white"
        >
          <CheckCircle2 :size="10" /> 已下载
        </span>
        <span
          class="px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 backdrop-blur bg-yellow-500/90 text-white"
        >
          <BookmarkCheck :size="10" /> 已入库
        </span>
      </div>
    </div>

    <!-- 信息 -->
    <div class="p-2.5 relative">
      <div class="text-xs font-medium text-white ellipsis-2 min-h-[2.2rem]" :title="title">
        {{ title }}
      </div>
      <div class="text-[10px] text-muted mt-1 ellipsis-1">
        <span class="font-mono">{{ item.rj_code }}</span>
        <span v-if="item.circle_name"> · {{ item.circle_name }}</span>
      </div>
      <div
        v-if="item.tags.length"
        class="flex flex-wrap gap-1 mt-1.5"
        :class="inLibrary ? '' : 'pr-14'"
      >
        <span
          v-for="t in item.tags.slice(0, 4)"
          :key="t"
          class="px-1 py-0.5 rounded text-[9px] bg-bg-hover text-muted border border-bg-border"
        >
          {{ t }}
        </span>
      </div>
      <div class="flex items-center gap-2 mt-1.5 text-[10px] text-muted" :class="inLibrary ? '' : 'pr-14'">
        <span v-if="item.price">{{ item.price.toLocaleString() }}円</span>
        <span v-if="item.dl_count">販売 {{ item.dl_count.toLocaleString() }}</span>
        <span v-if="item.rating">★ {{ item.rating.toFixed(1) }}</span>
      </div>

      <!-- 入库按钮（未入库时显示在信息区右下角） -->
      <!-- 注意：卡片根元素是 <a>，浏览器点击链接内的按钮会触发链接默认导航，
           必须 .prevent 阻止默认行为，否则点击入库会跳到预览页 -->
      <button
        v-if="!inLibrary"
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
