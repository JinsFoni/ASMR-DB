<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { Check, ImageOff, Loader2 } from "lucide-vue-next";
import type { WorkView } from "../../lib/types";

const props = defineProps<{
  view: WorkView;
  selected?: boolean;
  selectMode?: boolean;
  /** poster: 3:4 竖版海报（默认，裁切）；cover: 完整横版封面（不裁切） */
  variant?: "poster" | "cover";
}>();
const emit = defineEmits<{ (e: "toggle-select", id: number): void }>();

const wide = computed(() => props.variant === "cover");

/** 选择模式下点击卡片切换选中，而非跳转详情 */
function onCardClick(e: MouseEvent) {
  if (props.selectMode) {
    e.preventDefault();
    emit("toggle-select", props.view.work.id);
  }
}

const downloaded = computed(() => props.view.download_status === "downloaded");
const downloading = computed(() => props.view.download_status === "downloading");

const title = computed(
  () => props.view.work.title_zh || props.view.work.title_ja || props.view.work.rj_code
);
const circle = computed(() => props.view.work.circle_name || "");
const actorsText = computed(() => props.view.actors.join(" / "));

const dateText = computed(() => props.view.work.sale_date || "");
</script>

<template>
  <RouterLink
    :to="`/work/${view.work.id}`"
    class="work-card"
    :class="{ 'cursor-pointer': selectMode }"
    @click="onCardClick"
  >
    <!-- 封面（海报模式高度由 WorksCover 注入的 --poster-img-h 控制，与横版卡片严格等高） -->
    <div class="cover" :class="wide ? '' : 'poster'">
      <img
        v-if="view.work.cover_url"
        :src="view.work.cover_url"
        :alt="title"
        loading="lazy"
      />
      <div v-else class="w-full h-full flex items-center justify-center" style="color: var(--label-3)">
        <ImageOff :size="28" />
      </div>

      <!-- 选择模式复选框 -->
      <div
        v-if="selectMode"
        class="select-box"
        :class="{ on: selected }"
      >
        <Check v-if="selected" :size="14" />
      </div>

      <!-- 状态角标（设计稿 dl-badge 胶囊） -->
      <span v-if="downloaded && !selectMode" class="dl-badge">已下载</span>
      <span
        v-else-if="downloading && !selectMode"
        class="dl-badge dl-badge-downloading flex items-center gap-1"
      >
        <Loader2 :size="10" class="animate-spin" /> 下载中
      </span>
    </div>

    <!-- 信息 -->
    <h3 :title="title">{{ title }}</h3>
    <p class="wmeta">
      <span class="font-mono">{{ view.work.rj_code }}</span>
      <span v-if="actorsText" class="vas">{{ actorsText }}</span>
    </p>
    <p v-if="dateText || circle" class="wmeta2">
      <span class="font-mono">{{ dateText }}</span>
      <span v-if="circle" class="circle">{{ circle }}</span>
    </p>
    <div v-if="view.tags.length" class="wtags">
      <span v-for="t in view.tags.slice(0, 3)" :key="t.id" class="wtag">{{ t.name }}</span>
    </div>
  </RouterLink>
</template>

<style scoped>
.work-card {
  display: block;
  cursor: pointer;
  min-width: 0;
  transition: transform 0.25s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.work-card:hover {
  transform: translateY(-4px);
}
.cover {
  position: relative;
  aspect-ratio: 4 / 3;
  border-radius: var(--r-lg);
  overflow: hidden;
  background: var(--bg-secondary);
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.07);
}
.cover.poster {
  aspect-ratio: auto;
  height: var(--poster-img-h, 251px);
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transition: transform 0.4s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.work-card:hover .cover img {
  transform: scale(1.05);
}
.select-box {
  position: absolute;
  top: 10px;
  left: 10px;
  z-index: 10;
  width: 22px;
  height: 22px;
  border-radius: 8px;
  border: 2px solid rgba(255, 255, 255, 0.6);
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}
.select-box.on {
  background: var(--blue);
  border-color: var(--blue);
}
.dl-badge {
  position: absolute;
  top: 10px;
  right: 10px;
  font-size: 10.5px;
  font-weight: 700;
  color: #fff;
  background: rgba(48, 209, 88, 0.92);
  backdrop-filter: blur(6px);
  border-radius: 999px;
  padding: 3px 9px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
}
.dl-badge-downloading {
  background: rgba(47, 143, 239, 0.92);
}
h3 {
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.42;
  margin-top: 11px;
  letter-spacing: -0.01em;
  color: var(--label);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.work-card:hover h3 {
  color: var(--blue-deep);
}
.wmeta {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  color: var(--label-2);
  margin-top: 4px;
}
.wmeta .font-mono {
  flex-shrink: 0;
}
.vas {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}
.wmeta2 {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 1px;
}
.wmeta2 .circle {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}
.wtags {
  display: flex;
  gap: 5px;
  flex-wrap: wrap;
  margin-top: 8px;
}
.wtag {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--label-2);
  background: var(--fill-1);
  border-radius: 6px;
  padding: 2px 8px;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
