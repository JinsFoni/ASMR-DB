<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter, useRoute } from "vue-router";
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
  Expand,
} from "lucide-vue-next";
import { usePlayerStore } from "../../stores/player";

const player = usePlayerStore();
const router = useRouter();
const route = useRoute();

const speeds = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const showSpeeds = ref(false);

const fmt = (s: number) => {
  if (!isFinite(s) || s < 0) s = 0;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  return `${m}:${String(sec).padStart(2, "0")}`;
};

const progressPercent = computed(() => {
  if (player.duration <= 0) return 0;
  return (player.currentTime / player.duration) * 100;
});

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
</script>

<template>
  <footer
    v-if="player.visible && route.name !== 'player'"
    class="h-16 shrink-0 border-t border-bg-border bg-bg-card flex items-center gap-4 px-4 relative"
  >
    <!-- 进度条（悬浮在底部） -->
    <div
      class="absolute bottom-0 left-0 right-0 h-1 bg-bg-border cursor-pointer group"
      @click="onSeekClick"
    >
      <div
        class="h-full bg-accent transition-[width]"
        :style="{ width: progressPercent + '%' }"
      />
    </div>

    <!-- 当前曲目信息 -->
    <div class="w-48 shrink-0 min-w-0">
      <div class="text-xs text-white ellipsis-1">
        {{ player.currentTrack?.title || "未播放" }}
      </div>
      <div class="text-[10px] text-muted ellipsis-1">
        {{ player.work?.work.title_ja || player.work?.work.rj_code || "" }}
      </div>
    </div>

    <!-- 控制按钮 -->
    <div class="flex items-center gap-1.5">
      <button class="btn-ghost !p-1.5" title="上一首" @click="player.prev()">
        <SkipBack :size="16" />
      </button>
      <button
        class="w-9 h-9 rounded-full bg-accent hover:bg-accent-light text-white flex items-center justify-center transition-colors"
        :title="player.playing ? '暂停' : '播放'"
        @click="player.togglePlay()"
      >
        <Loader2 v-if="player.buffering" :size="16" class="animate-spin" />
        <Pause v-else-if="player.playing" :size="16" />
        <Play v-else :size="16" class="ml-0.5" />
      </button>
      <button class="btn-ghost !p-1.5" title="下一首" @click="player.next()">
        <SkipForward :size="16" />
      </button>
    </div>

    <!-- 时间 -->
    <div class="text-[11px] text-muted tabular-nums w-28 shrink-0">
      {{ fmt(player.currentTime) }} / {{ fmt(player.duration) }}
    </div>

    <div class="flex-1" />

    <!-- 倍速 -->
    <div class="relative">
      <button class="btn-outline !px-2 text-xs" @click="showSpeeds = !showSpeeds">
        {{ player.rate.toFixed(2) }}x
      </button>
      <div
        v-if="showSpeeds"
        class="absolute bottom-9 right-0 card p-1 z-50 shadow-xl"
      >
        <button
          v-for="s in speeds"
          :key="s"
          class="w-full text-left px-3 py-1 text-xs rounded-md"
          :class="player.rate === s ? 'text-accent-light bg-bg-hover' : 'text-muted hover:text-white hover:bg-bg-hover'"
          @click="player.setRate(s); showSpeeds = false"
        >
          {{ s.toFixed(2) }}x
        </button>
      </div>
    </div>

    <!-- 循环模式 -->
    <button class="btn-ghost !p-1.5" :title="'循环模式: ' + player.loopMode" @click="cycleLoop">
      <Repeat1 v-if="player.loopMode === 'one'" :size="16" class="text-accent-light" />
      <Repeat v-else :size="16" :class="player.loopMode === 'all' ? 'text-accent-light' : ''" />
    </button>

    <!-- 音量 -->
    <div class="flex items-center gap-1.5 w-28">
      <button class="btn-ghost !p-1" @click="player.setVolume(player.volume > 0 ? 0 : 0.8)">
        <VolumeX v-if="player.volume === 0" :size="15" />
        <Volume2 v-else :size="15" />
      </button>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        :value="player.volume"
        class="flex-1 accent-accent"
        @input="player.setVolume(parseFloat(($event.target as HTMLInputElement).value))"
      />
    </div>

    <!-- 展开全屏播放器 -->
    <button class="btn-ghost !p-1.5" title="全屏播放器（字幕）" @click="router.push({ name: 'player' })">
      <Expand :size="16" />
    </button>
  </footer>
</template>
