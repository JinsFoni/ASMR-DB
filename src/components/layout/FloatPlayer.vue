<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Repeat,
  Repeat1,
  Loader2,
  Maximize2,
  PanelBottom,
  ImageOff,
} from "lucide-vue-next";
import { usePlayerStore } from "../../stores/player";
import { imgUrl } from "../../lib/api";

/**
 * 右侧悬浮播放器（float 模式）：与底部播放栏（dock 模式）共用全局播放器状态，
 * 通过 player.toggleFloatMode() 互相切换。隐藏播放时仍继续，只是形态变化。
 */
const player = usePlayerStore();
const router = useRouter();
const route = useRoute();

const coverUrl = computed(() => imgUrl(player.work?.work.cover_url) || "");
const trackTitle = computed(() => player.currentTrack?.title || "");
const workTitle = computed(
  () => player.work?.work.title_zh || player.work?.work.title_ja || player.work?.work.rj_code || ""
);

const progressPercent = computed(() => {
  if (player.duration <= 0) return 0;
  return (player.currentTime / player.duration) * 100;
});

const fmt = (s: number) => {
  if (!isFinite(s) || s < 0) s = 0;
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
};

function onSeekClick(e: MouseEvent) {
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  const ratio = (e.clientX - rect.left) / rect.width;
  player.seek(ratio * player.duration);
}

function cycleLoop() {
  player.loopMode =
    player.loopMode === "none" ? "all" : player.loopMode === "all" ? "one" : "none";
}

/** 回到全屏播放器页：先退出悬浮模式，底部栏在播放器页本来就不显示 */
function goFullscreen() {
  player.setFloatMode(false);
  router.push({ name: "player" });
}</script>

<template>
  <aside v-if="player.visible && player.floatMode && route.name !== 'player'" class="fplayer glass">
    <!-- 封面：原图比例，不裁切 -->
    <div class="fp-art">
      <img v-if="coverUrl" :src="coverUrl" alt="" />
      <div v-else class="fp-art-empty">
        <ImageOff :size="26" />
      </div>
      <div v-if="player.playing" class="fp-eq" aria-hidden="true">
        <i /><i /><i />
      </div>
    </div>

    <!-- 标题 -->
    <div class="fp-meta">
      <b :title="trackTitle">{{ trackTitle || "未播放" }}</b>
      <span :title="workTitle">{{ workTitle }}</span>
    </div>

    <!-- 字幕行：固定 2 行高度，1 行时也不改变卡片高度 -->
    <p v-if="player.subtitleCues.length" class="fp-sub">{{ player.subtitleText }}</p>

    <!-- 进度 -->
    <div class="fp-prog">
      <div class="fp-bar" @click="onSeekClick">
        <i :style="{ width: progressPercent + '%' }" />
      </div>
      <div class="fp-time">
        <span>{{ fmt(player.currentTime) }}</span>
        <span>{{ fmt(player.duration) }}</span>
      </div>
    </div>

    <!-- 控制 -->
    <div class="fp-ctrls">
      <button class="fp-ctrl" :title="'循环模式: ' + player.loopMode" @click="cycleLoop">
        <Repeat1 v-if="player.loopMode === 'one'" :size="15" class="text-accent-light" />
        <Repeat v-else :size="15" :class="player.loopMode === 'all' ? 'text-accent-light' : ''" />
      </button>
      <button class="fp-ctrl" title="上一曲" @click="player.prev()">
        <SkipBack :size="17" />
      </button>
      <button class="fp-main" :title="player.playing ? '暂停' : '播放'" @click="player.togglePlay()">
        <Loader2 v-if="player.buffering" :size="19" class="animate-spin" />
        <Pause v-else-if="player.playing" :size="19" />
        <Play v-else :size="19" class="ml-0.5" />
      </button>
      <button class="fp-ctrl" title="下一曲" @click="player.next()">
        <SkipForward :size="17" />
      </button>
      <button class="fp-ctrl" @click="player.setVolume(player.volume > 0 ? 0 : 0.8)">
        <VolumeX v-if="player.volume === 0" :size="14" />
        <Volume2 v-else :size="14" />
      </button>
    </div>

    <!-- 底部操作：左下角切回底部播放栏，右下角进全屏播放器页 -->
    <div class="fp-actions">
      <button class="fp-action" title="切回底部播放栏" @click="player.setFloatMode(false)">
        <PanelBottom :size="13" />
        <span>底部播放栏</span>
      </button>
      <button class="fp-action" title="全屏播放器（字幕）" @click="goFullscreen()">
        <Maximize2 :size="13" />
        <span>全屏播放器</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.fplayer {
  --fp-w: 260px;
  position: fixed;
  /* 右侧垂直居中 */
  top: 50%;
  transform: translateY(-50%);
  /* 水平居中于「内容网格右缘 ↔ 浏览器窗口右缘」的空带，两侧留白相等：
     内容容器 min(1280px, 视口-滚动条槽) 居中，网格右缘距窗口右缘
     (100vw-1280px)/2 + 槽宽/2 + 28px，卡片在该空带内对半分；
     空带放不下卡片时退回与内容网格右缘对齐 */
  right: max(
    calc(var(--app-scrollbar-w, 0px) + 28px),
    calc(
      (100vw - 1280px) / 4 + var(--app-scrollbar-w, 0px) / 4 + 14px -
        var(--fp-w) / 2
    )
  );
  z-index: 70;
  width: var(--fp-w);
  border-radius: 22px;
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.8),
    inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
  padding: 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 11px;
}
html[data-theme="light"] .fplayer,
html[data-theme="sakura"] .fplayer {
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.25),
    inset 0 0 0 0.5px rgba(0, 0, 0, 0.06);
}

/* 收起按钮：右上角小圆钮 */
.fp-close {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 50%;
  background: var(--fill-2);
  color: var(--label-2);
  cursor: pointer;
  display: grid;
  place-items: center;
  opacity: 0;
  transition: opacity 0.15s, background 0.15s;
}
.fplayer:hover .fp-close {
  opacity: 1;
}
.fp-close:hover {
  background: var(--fill-3);
  color: var(--label);
}

/* 封面：保持原图比例（不裁切），宽度撑满卡片 */
.fp-art {
  position: relative;
  width: 100%;
  border-radius: 14px;
  overflow: hidden;
  background: var(--fill-2);
  box-shadow: 0 8px 24px -8px rgba(0, 0, 0, 0.45);
}
.fp-art img {
  width: 100%;
  height: auto;
  display: block;
}
.fp-art-empty {
  width: 100%;
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  color: var(--label-3);
}
.fp-eq {
  position: absolute;
  left: 10px;
  bottom: 10px;
  display: flex;
  align-items: flex-end;
  gap: 3px;
  height: 14px;
}
.fp-eq i {
  width: 3px;
  border-radius: 2px;
  background: #fff;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  animation: fpeq 0.9s ease-in-out infinite;
}
.fp-eq i:nth-child(2) {
  animation-delay: 0.25s;
}
.fp-eq i:nth-child(3) {
  animation-delay: 0.5s;
}
@keyframes fpeq {
  0%, 100% { height: 4px; }
  50% { height: 14px; }
}

/* 标题 */
.fp-meta {
  width: 100%;
  min-width: 0;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.fp-meta b {
  font-size: 13px;
  line-height: 1.35;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fp-meta span {
  font-size: 11px;
  color: var(--label-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 字幕行：始终占 2 行高度，避免字幕换行时卡片高度跳变 */
.fp-sub {
  width: 100%;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--label-2);
  text-align: center;
  height: 3em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

/* 进度 */
.fp-prog {
  width: 100%;
}
.fp-bar {
  height: 12px;
  display: flex;
  align-items: center;
  cursor: pointer;
  position: relative;
}
.fp-bar::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 4px;
  border-radius: 2px;
  background: var(--fill-2);
}
.fp-bar i {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: 2px;
  background: var(--label);
}
.fp-time {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  font-weight: 600;
  color: var(--label-2);
  font-variant-numeric: tabular-nums;
  margin-top: 2px;
}

/* 控制按钮 */
.fp-ctrls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
.fp-ctrl {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--label);
  display: grid;
  place-items: center;
  opacity: 0.75;
  transition: opacity 0.15s, transform 0.15s;
  padding: 0;
}
.fp-ctrl:hover {
  opacity: 1;
  transform: scale(1.08);
}
.fp-main {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: #fff;
  color: #0d0d0f;
  border: none;
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: transform 0.15s, background 0.15s;
}
.fp-main:hover {
  transform: scale(1.07);
  background: var(--blue);
  color: #fff;
}

/* 底部双操作：左右分布 */
.fp-actions {
  width: 100%;
  display: flex;
  justify-content: space-between;
  gap: 8px;
}
.fp-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: none;
  background: var(--fill-1);
  color: var(--label-2);
  font-size: 11px;
  font-weight: 600;
  border-radius: 999px;
  height: 26px;
  padding: 0 11px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.fp-action:hover {
  background: var(--fill-3);
  color: var(--label);
}

@media (max-width: 900px) {
  .fplayer {
    display: none;
  }
}
</style>
