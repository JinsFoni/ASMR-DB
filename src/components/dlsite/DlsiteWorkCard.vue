<script setup lang="ts">
import { computed } from "vue";
import { CheckCircle2, ImageOff, Loader2 } from "lucide-vue-next";
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

/** 排名角标配色：前三名金/银/铜渐变，其余灰色填充（设计稿 .rank-no） */
const rankMeta = computed(() => {
  switch (props.item.rank) {
    case 1:
      return "p1";
    case 2:
      return "p2";
    case 3:
      return "p3";
    default:
      return "";
  }
});

const title = computed(() => props.item.title || props.item.rj_code);
</script>

<template>
  <article
    class="rank-card"
    @click="emit('open', item)"
  >
    <!-- 排名角标（左上角悬浮） -->
    <div class="rank-no" :class="rankMeta" :title="`排行榜第 ${item.rank} 名`">
      {{ item.rank }}
    </div>

    <div class="rc-main">
      <!-- 封面（固定宽，原始比例不裁切） -->
      <div class="rc-cover">
        <img
          v-if="item.cover_url"
          :src="item.cover_url"
          :alt="title"
          loading="lazy"
        />
        <div v-else class="w-full aspect-[4/3] flex items-center justify-center" style="color: var(--label-3)">
          <ImageOff :size="28" />
        </div>
      </div>

      <div class="rc-body">
        <h3 :title="title">{{ title }}</h3>
        <div v-if="item.tags.length" class="rc-tags">
          <span v-for="t in item.tags.slice(0, 3)" :key="t" class="wtag">{{ t }}</span>
        </div>
        <p v-if="item.circle_name" class="rc-meta">{{ item.circle_name }}</p>
        <p class="rc-meta2">
          <span class="font-mono">{{ item.rj_code }}</span>
        </p>
      </div>
    </div>

    <!-- 右下：已入库 / 入库按钮（设计稿 .rc-side） -->
    <div class="rc-side">
      <template v-if="inLibrary">
        <span v-if="item.download_status === 'downloaded'" class="inlib" style="color: var(--green)">
          <CheckCircle2 :size="11" /> 已入库 · 已下载
        </span>
        <span v-else class="inlib"><CheckCircle2 :size="11" /> 已入库</span>
      </template>
      <!-- 卡片根元素不是链接，但仍 .stop 防止冒泡触发卡片 open -->
      <button
        v-else
        class="add-btn"
        :disabled="adding"
        title="将此作品加入作品库"
        @click.stop="emit('add', item)"
      >
        <Loader2 v-if="adding" :size="11" class="animate-spin" />
        <template v-else>＋ 入库</template>
      </button>
    </div>
  </article>
</template>

<style scoped>
.rank-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-secondary);
  border-radius: var(--r-lg);
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.06);
  padding: 14px;
  cursor: pointer;
  transition: transform 0.22s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.rank-card:hover {
  transform: translateY(-3px);
}
.rank-no {
  position: absolute;
  top: -6px;
  left: -6px;
  z-index: 2;
  min-width: 30px;
  height: 30px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 13.5px;
  font-weight: 800;
  color: #fff;
  background: var(--fill-3);
  box-shadow: 0 4px 12px -3px rgba(0, 0, 0, 0.6);
  font-variant-numeric: tabular-nums;
}
.rank-no.p1 {
  background: linear-gradient(140deg, #ffd60a, #ff9f0a);
  color: #402a00;
}
.rank-no.p2 {
  background: linear-gradient(140deg, #e8e8ed, #a8a8b0);
  color: #2a2a2e;
}
.rank-no.p3 {
  background: linear-gradient(140deg, #e0a86a, #b3703a);
  color: #3a2008;
}

.rc-main {
  display: flex;
  gap: 14px;
  min-width: 0;
}
.rc-cover {
  width: 160px;
  border-radius: 12px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--fill-1);
  align-self: flex-start;
}
.rc-cover img {
  width: 100%;
  height: auto;
  display: block;
}
.rc-body {
  flex: 1;
  min-width: 0;
}
.rc-body h3 {
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.45;
  letter-spacing: -0.01em;
  color: var(--label);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.rank-card:hover h3 {
  color: var(--blue-deep);
}
.rc-tags {
  display: flex;
  gap: 5px;
  flex-wrap: nowrap;
  overflow: hidden;
}
.wtag {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--label-2);
  background: var(--fill-1);
  border-radius: 6px;
  padding: 2px 8px;
  white-space: nowrap;
}
.rc-meta {
  font-size: 11.5px;
  color: var(--label-2);
  margin-top: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.rc-meta2 {
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 3px;
}

.rc-side {
  position: absolute;
  right: 14px;
  bottom: 12px;
}
.inlib {
  font-size: 11px;
  font-weight: 700;
  color: #4cd964;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.add-btn {
  height: 28px;
  padding: 0 13px;
  border: none;
  border-radius: 999px;
  background: var(--blue);
  color: #fff;
  font-family: inherit;
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  transition: background 0.2s;
}
.add-btn:hover {
  background: var(--blue-deep);
}
.add-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
