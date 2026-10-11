<script setup lang="ts">
import { computed } from "vue";
import { BookmarkCheck, Captions, ImageOff, Loader2 } from "lucide-vue-next";
import { imgUrl } from "../../lib/api";
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
const coverSrc = computed(() => imgUrl(props.item.cover_url));
const title = computed(() => props.item.title || props.item.rj_code || `#${props.item.id}`);

const metaText = computed(() => {
  if (props.item.rj_code) {
    const parts = [props.item.rj_code];
    if (props.item.circle_name) parts.push(props.item.circle_name);
    return parts.join(" · ");
  }
  return "";
});
const meta2Text = computed(() => {
  const parts: string[] = [];
  if (props.item.release) parts.push(props.item.release.slice(0, 10));
  if (props.item.duration_sec) parts.push(`${Math.round(props.item.duration_sec / 60)} 分钟`);
  if (!parts.length && props.item.dl_count) parts.push(`販売 ${props.item.dl_count.toLocaleString()}`);
  return parts.join(" · ");
});
</script>

<template>
  <article
    class="oc"
    @click="emit('open', item)"
  >
    <!-- 封面（4:3 裁切，hover 缩放） -->
    <div class="oc-cover">
      <img
        v-if="coverSrc"
        :src="coverSrc"
        :alt="title"
        loading="lazy"
      />
      <div v-else class="w-full h-full flex items-center justify-center" style="color: var(--label-3)">
        <ImageOff :size="28" />
      </div>

      <!-- 字幕标识（封面左下角） -->
      <span v-if="item.has_subtitle" class="sub-badge" title="该作品带字幕文件">
        <Captions :size="11" /> 字幕
      </span>

      <!-- 已入库角标（设计稿 .corner） -->
      <span v-if="localId" class="corner"><BookmarkCheck :size="10" /> 已入库</span>
    </div>

    <h3 :title="title">{{ title }}</h3>
    <p class="oc-meta">{{ metaText }}</p>
    <p v-if="meta2Text" class="oc-meta2">{{ meta2Text }}</p>
    <div v-if="item.tags.length" class="oc-tags">
      <span v-for="t in item.tags.slice(0, 3)" :key="t" class="wtag">{{ t }}</span>
    </div>

    <!-- 入库按钮（未入库且有 RJ 号，悬浮于信息区右下） -->
    <button
      v-if="!localId && item.rj_code"
      class="add-btn"
      :disabled="adding"
      title="将此作品加入作品库"
      @click.stop="emit('add', item)"
    >
      <Loader2 v-if="adding" :size="11" class="animate-spin" />
      <template v-else>＋ 入库</template>
    </button>
  </article>
</template>

<style scoped>
.oc {
  position: relative;
  cursor: pointer;
  transition: transform 0.25s cubic-bezier(0.3, 0.6, 0.2, 1);
  min-width: 0;
}
.oc:hover {
  transform: translateY(-4px);
}
.oc-cover {
  position: relative;
  aspect-ratio: 4 / 3;
  border-radius: var(--r-lg);
  overflow: hidden;
  background: var(--bg-secondary);
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.07);
}
.oc-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  transition: transform 0.4s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.oc:hover .oc-cover img {
  transform: scale(1.05);
}
.corner {
  position: absolute;
  top: 10px;
  right: 10px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  font-weight: 700;
  color: #fff;
  background: rgba(48, 209, 88, 0.92);
  backdrop-filter: blur(6px);
  border-radius: 999px;
  padding: 3px 9px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
}
.sub-badge {
  position: absolute;
  left: 10px;
  bottom: 10px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  font-weight: 700;
  color: #fff;
  background: rgba(28, 28, 32, 0.72);
  backdrop-filter: blur(6px);
  border-radius: 999px;
  padding: 3px 9px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5), inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
}
.oc h3 {
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
.oc:hover h3 {
  color: var(--blue-deep);
}
.oc-meta {
  font-size: 11.5px;
  color: var(--label-2);
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.oc-meta2 {
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 1px;
}
.oc-tags {
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
.add-btn {
  position: absolute;
  right: 0;
  bottom: -4px;
  height: 27px;
  padding: 0 12px;
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
  z-index: 2;
}
.add-btn:hover {
  background: var(--blue-deep);
}
.add-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
