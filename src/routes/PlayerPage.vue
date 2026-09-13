<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
  ArrowLeft,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Repeat,
  Repeat1,
  Captions,
  CaptionsOff,
  Monitor,
  Loader2,
  ListMusic,
} from "lucide-vue-next";
import { usePlayerStore } from "../stores/player";

const router = useRouter();
const player = usePlayerStore();

const speeds = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const showSpeeds = ref(false);

const coverUrl = computed(() => player.work?.work.cover_url || "");
const trackTitle = computed(() => player.currentTrack?.title || "");
const workTitle = computed(
  () => player.work?.work.title_ja || player.work?.work.rj_code || ""
);

const progressPercent = computed(() => {
  if (player.duration <= 0) return 0;
  return (player.currentTime / player.duration) * 100;
});

const fmt = (s: number) => {
  if (!isFinite(s) || s < 0) s = 0;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  return `${m}:${String(sec).padStart(2, "0")}`;
};

const fmtShort = (s: number) => {
  if (!isFinite(s) || s < 0) s = 0;
  return `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
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

onMounted(() => {
  // 没有活跃播放时回到首页
  if (!player.visible) {
    router.replace({ name: "home" });
  }
});
</script>

<template>
  <div class="h-full flex flex-col bg-bg min-h-0">
    <!-- 顶部栏 -->
    <div class="flex items-center gap-2 px-4 py-3 border-b border-bg-border shrink-0">
      <button class="btn-ghost !p-2" title="返回" @click="router.back()">
        <ArrowLeft :size="18" />
      </button>
      <div class="flex-1 min-w-0">
        <div class="text-sm font-medium text-white ellipsis-1">{{ trackTitle || "未播放" }}</div>
        <div class="text-[11px] text-muted ellipsis-1">{{ workTitle }}</div>
      </div>
      <!-- 字幕切换（默认关闭） -->
      <button
        class="btn-ghost !p-2"
        :class="player.subtitleEnabled ? 'text-accent-light' : ''"
        :title="player.subtitleEnabled ? '关闭字幕' : '开启字幕'"
        @click="player.toggleSubtitle()"
      >
        <Captions v-if="player.subtitleEnabled" :size="18" />
        <CaptionsOff v-else :size="18" />
      </button>
      <!-- 桌面悬浮字幕 -->
      <button
        class="btn-ghost !p-2"
        :class="player.desktopSubtitleEnabled ? 'text-accent-light' : ''"
        :title="player.desktopSubtitleEnabled ? '关闭桌面悬浮字幕' : '开启桌面悬浮字幕（类似桌面歌词）'"
        @click="player.toggleDesktopSubtitle()"
      >
        <Monitor :size="18" />
      </button>
    </div>

    <!-- 主体：封面 + 字幕 -->
    <div class="flex-1 flex flex-col items-center justify-center min-h-0 overflow-y-auto px-8 py-6">
      <img
        v-if="coverUrl"
        :src="coverUrl"
        class="w-52 md:w-64 aspect-[3/4] object-cover rounded-2xl shadow-2xl bg-bg-hover border border-bg-border"
      />
      <div
        v-else
        class="w-52 md:w-64 aspect-[3/4] rounded-2xl bg-bg-hover border border-bg-border flex items-center justify-center text-muted/40"
      >
        <ListMusic :size="48" />
      </div>

      <!-- 字幕显示区 -->
      <div class="mt-8 w-full max-w-2xl min-h-24 flex flex-col items-center justify-center">
        <div
          v-if="player.subtitleEnabled && player.subtitleText"
          class="text-center px-6 py-3 rounded-2xl bg-black/50 backdrop-blur-sm max-w-full"
        >
          <p class="text-white text-lg md:text-xl leading-relaxed whitespace-pre-line">
            {{ player.subtitleText }}
          </p>
          <p v-if="player.subtitleName" class="text-[10px] text-white/40 mt-1">
            {{ player.subtitleName }}
          </p>
        </div>
        <div v-else-if="player.subtitleEnabled" class="text-xs text-muted/50">
          {{ player.subtitleLoading ? "正在加载字幕…" : (player.subtitleCues.length ? "（等待字幕时间点）" : "未找到字幕文件 (.vtt / .srt / .lrc)") }}
        </div>
      </div>
    </div>

    <!-- 底部控制区 -->
    <div class="shrink-0 px-6 pb-4 pt-2 space-y-3">
      <!-- 进度条（下方） -->
      <div
        class="w-full h-2 bg-bg-border rounded-full cursor-pointer group"
        @click="onSeekClick"
      >
        <div
          class="h-full bg-accent rounded-full group-hover:bg-accent-light transition-[width]"
          :style="{ width: progressPercent + '%' }"
        />
      </div>

      <!-- 时间 -->
      <div class="flex justify-between text-[11px] text-muted tabular-nums">
        <span>{{ fmt(player.currentTime) }}</span>
        <span>{{ fmt(player.duration) }}</span>
      </div>

      <!-- 主控制 -->
      <div class="flex items-center justify-center gap-5">
        <button class="btn-ghost !p-2" title="上一首" @click="player.prev()">
          <SkipBack :size="22" />
        </button>
        <button
          class="w-14 h-14 rounded-full bg-accent hover:bg-accent-light text-white flex items-center justify-center transition-colors"
          :title="player.playing ? '暂停' : '播放'"
          @click="player.togglePlay()"
        >
          <Loader2 v-if="player.buffering" :size="24" class="animate-spin" />
          <Pause v-else-if="player.playing" :size="24" />
          <Play v-else :size="24" class="ml-0.5" />
        </button>
        <button class="btn-ghost !p-2" title="下一首" @click="player.next()">
          <SkipForward :size="22" />
        </button>
      </div>

      <!-- 次级控制 -->
      <div class="flex items-center justify-between">
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

        <div class="relative">
          <button class="btn-outline !px-2 text-xs" @click="showSpeeds = !showSpeeds">
            {{ player.rate.toFixed(2) }}x
          </button>
          <div v-if="showSpeeds" class="absolute bottom-8 right-0 card p-1 z-50 shadow-xl">
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

        <button
          class="btn-ghost !p-1.5"
          :title="'循环模式: ' + player.loopMode"
          @click="cycleLoop"
        >
          <Repeat1 v-if="player.loopMode === 'one'" :size="16" class="text-accent-light" />
          <Repeat v-else :size="16" :class="player.loopMode === 'all' ? 'text-accent-light' : ''" />
        </button>
      </div>
    </div>

    <!-- 播放列表 -->
    <div class="shrink-0 border-t border-bg-border max-h-52 overflow-y-auto">
      <div
        v-for="(t, i) in player.playlist"
        :key="t.id"
        class="flex items-center gap-3 px-5 py-2 text-xs hover:bg-bg-hover cursor-pointer transition-colors"
        :class="i === player.currentIndex ? 'bg-accent/10 text-accent-light' : 'text-white'"
        @click="player.playTrack(i)"
      >
        <span class="w-6 text-center font-mono text-muted shrink-0">
          {{ String(t.track_number || i + 1).padStart(2, "0") }}
        </span>
        <span class="flex-1 min-w-0 ellipsis-1">
          {{ t.title || t.file_path.split(/[\\/]/).pop() }}
        </span>
        <span class="text-muted tabular-nums shrink-0">{{ fmtShort(t.duration_sec || 0) }}</span>
      </div>
    </div>
  </div>
</template>
