<script setup lang="ts">
import { computed, ref } from "vue";
import { BookmarkCheck, Captions, Heart, Loader2, Pause, Play, Star } from "lucide-vue-next";
import { imgUrl } from "../../lib/api";
import type { AsmrOnlineWork } from "../../lib/types";

const props = defineProps<{
  item: AsmrOnlineWork;
  /** 本地库中已存在的本地作品 id（用于已入库标识与跳转） */
  localWorkId?: number | null;
  /** 该卡片正在执行入库 */
  adding?: boolean;
  /** 有音频可播（本地已入库有音轨，或在线时长已知）；缺省 false 不显示按钮 */
  playable?: boolean;
  /** 正在拉取音轨 */
  playLoading?: boolean;
  /** 该作品已加载进播放器（图标切暂停） */
  playCurrent?: boolean;
}>();

const emit = defineEmits<{
  (e: "open", item: AsmrOnlineWork): void;
  (e: "add", item: AsmrOnlineWork): void;
  (e: "play", item: AsmrOnlineWork): void;
  (e: "favorite", item: AsmrOnlineWork, favorite: boolean, done: (ok: boolean) => void): void;
}>();

const localId = computed(() => props.localWorkId ?? props.item.local_work_id ?? null);
const coverSrc = computed(() => imgUrl(props.item.cover_url));
const title = computed(() => props.item.title || props.item.rj_code || `#${props.item.id}`);

/* 封面缺失或加载失败（asmr.one 部分作品源站没有封面图，代理返回 502）时用默认图 */
const coverFailed = ref(false);
const DEFAULT_COVER = "/default-cover.png";

/* 元数据结构与作品库卡片（WorkCard）一致：RJ 号+声优 / 发售日期+数量 */
const actorsText = computed(() => props.item.vas.join(" / "));
const dateText = computed(() => props.item.release?.slice(0, 10) || "");
const wishlistText = computed(() =>
  props.item.wishlist_count != null ? props.item.wishlist_count.toLocaleString() : ""
);

/* 收藏到 asmr.one 账号：父组件负责请求，完成后回调回滚状态 */
const marked = computed(() => !!props.item.user_marked);
const favLoading = ref(false);
function toggleFavorite() {
  if (favLoading.value) return;
  const next = !marked.value;
  // 乐观更新：立即切换图标，父组件请求失败后经 done(false) 回滚
  props.item.user_marked = next;
  favLoading.value = true;
  emit("favorite", props.item, next, (ok) => {
    if (!ok) props.item.user_marked = !next;
    favLoading.value = false;
  });
}
</script>

<template>
  <article
    class="oc"
    @click="emit('open', item)"
  >
    <!-- 封面（4:3 裁切，hover 缩放） -->
    <div class="oc-cover">
      <img
        v-if="coverSrc && !coverFailed"
        :src="coverSrc"
        :alt="title"
        loading="lazy"
        @error="coverFailed = true"
      />
      <img v-else :src="DEFAULT_COVER" :alt="title" class="default-cover" />

      <!-- 左上角：收藏到 asmr.one 账号 -->
      <button
        v-if="item.id"
        class="fav-btn"
        :class="{ on: marked }"
        :title="marked ? '取消收藏（asmr.one）' : '收藏到 asmr.one 账号'"
        @click.stop="toggleFavorite"
      >
        <Loader2 v-if="favLoading" :size="13" class="animate-spin" />
        <Heart v-else :size="13" class="fav-ico" />
      </button>

      <!-- 左下角：播放按钮 -->
      <div class="cover-bl">
        <button
          v-if="playable && !adding"
          class="play-btn"
          :title="playCurrent ? '暂停 / 继续' : '播放'"
          @click.stop="emit('play', item)"
        >
          <Loader2 v-if="playLoading" :size="13" class="animate-spin" />
          <Pause v-else-if="playCurrent" :size="14" />
          <Play v-else :size="14" class="play-ico" />
        </button>
      </div>

      <!-- 已入库角标（设计稿 .corner） -->
      <span v-if="localId" class="corner"><BookmarkCheck :size="10" /> 已入库</span>

      <!-- 右下角：字幕标识 + 评分角标 -->
      <div class="cover-br">
        <span v-if="item.has_subtitle" class="sub-badge" title="该作品带字幕文件">
          <Captions :size="11" /> 字幕
        </span>
        <span v-if="item.rating != null" class="rating-badge">
          <Star :size="10" class="rating-star" /> {{ item.rating }}
        </span>
      </div>
    </div>

    <h3 :title="title">{{ title }}</h3>
    <p class="oc-meta">
      <span v-if="item.rj_code" class="font-mono">{{ item.rj_code }}</span>
      <span v-if="actorsText" class="vas">{{ actorsText }}</span>
    </p>
    <p v-if="dateText || wishlistText" class="oc-meta2">
      <span class="font-mono">{{ dateText }}</span>
      <span v-if="wishlistText" class="wishlist" title="DLsite 收藏数">♥ {{ wishlistText }}</span>
    </p>
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
  color: #1c7c2e;
  background: rgba(255, 255, 255, 0.86);
  border-radius: 999px;
  padding: 3px 9px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
}
.cover-bl {
  position: absolute;
  left: 6px;
  bottom: 6px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.fav-btn {
  position: absolute;
  top: 6px;
  left: 6px;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 999px;
  background: rgba(20, 20, 24, 0.78);
  color: #fff;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
  transition: background 0.15s, transform 0.15s;
  z-index: 2;
}
.fav-btn:hover {
  background: var(--blue);
  transform: scale(1.08);
}
.fav-btn.on {
  background: rgba(20, 20, 24, 0.78);
}
.fav-btn.on .fav-ico {
  color: #ff375f;
  fill: #ff375f;
}
.play-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 999px;
  background: rgba(20, 20, 24, 0.78);
  color: #fff;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
  transition: background 0.15s, transform 0.15s;
}
.play-btn:hover {
  background: var(--blue);
  transform: scale(1.08);
}
.play-ico {
  margin-left: 1.5px; /* 视觉居中：三角形几何中心偏左 */
}
.sub-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  font-weight: 700;
  color: #333;
  background: rgba(255, 255, 255, 0.86);
  border-radius: 999px;
  padding: 3px 9px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5), inset 0 0 0 0.5px rgba(0, 0, 0, 0.12);
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
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  color: var(--label-2);
  margin-top: 4px;
}
.oc-meta .font-mono {
  flex-shrink: 0;
}
.vas {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}
.oc-meta2 {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 1px;
}
.oc-meta2 .font-mono {
  flex-shrink: 0;
}
.wishlist {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}
.cover-br {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.rating-badge {
  position: static;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  font-weight: 700;
  color: #333;
  background: rgba(255, 255, 255, 0.86);
  border-radius: 999px;
  padding: 3px 8px;
  box-shadow: 0 2px 8px -2px rgba(0, 0, 0, 0.5);
  font-variant-numeric: tabular-nums;
}
.rating-star {
  color: #ffd60a;
  fill: #ffd60a;
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
