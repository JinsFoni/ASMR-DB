<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { Play, Pause, Music, ExternalLink } from "lucide-vue-next";
import type { Track, WorkView } from "../../lib/types";
import { usePlayerStore } from "../../stores/player";
import * as api from "../../lib/api";

const props = defineProps<{ tracks: Track[]; work: WorkView }>();
const player = usePlayerStore();
const router = useRouter();

const fmtDuration = (s: number | null) => {
  if (!s) return "--:--";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
};

const isCurrent = computed(() => (t: Track) =>
  player.work?.work.id === props.work.work.id && player.currentTrack?.id === t.id
);

async function openWithPlayer(t: Track) {
  try {
    await api.openAudioFile(t.file_path);
  } catch (e) {
    alert(`打开失败: ${e}`);
  }
}

/** 播放指定音轨并进入全屏播放器页面。 */
function playTrack(i: number) {
  player.playFromWork(props.work, props.tracks, i);
  router.push({ name: "player" });
}
</script>

<template>
  <div class="card overflow-hidden">
    <div class="px-4 py-2.5 border-b border-bg-border flex items-center justify-between">
      <div class="text-xs font-semibold text-white">🎵 音轨列表 ({{ tracks.length }})</div>
      <span class="text-[10px] text-muted">{{ (props.work.total_duration_sec / 60).toFixed(0) }} 分钟</span>
    </div>

    <div v-if="!tracks.length" class="px-4 py-6 text-center text-xs text-muted">
      暂无音轨。下载并解压作品后会自动扫描。
    </div>

    <div class="max-h-96 overflow-y-auto">
      <div
        v-for="(t, i) in tracks"
        :key="t.id"
        class="w-full flex items-center gap-3 px-4 py-2 text-left hover:bg-bg-hover transition-colors group"
        :class="isCurrent(t) ? 'bg-accent/10' : ''"
        @dblclick="playTrack(i)"
      >
        <span class="w-6 text-center text-[11px] font-mono text-muted shrink-0">
          {{ String(t.track_number || i + 1).padStart(2, "0") }}
        </span>
        <button
          class="w-6 h-6 rounded-full flex items-center justify-center shrink-0 cursor-pointer"
          :class="isCurrent(t) ? 'bg-accent text-white' : 'bg-bg-hover text-muted group-hover:bg-bg-border group-hover:text-white'"
          :title="isCurrent(t) ? '当前播放' : '播放'"
          @click="playTrack(i)"
        >
          <Music v-if="!isCurrent(t)" :size="12" />
          <Play v-else-if="!player.playing" :size="12" />
          <Pause v-else :size="12" />
        </button>
        <div class="flex-1 min-w-0 cursor-pointer" :title="'双击播放：' + (t.title || t.file_path.split(/[\\/]/).pop())" @dblclick="playTrack(i)">
          <div class="text-xs ellipsis-1" :class="isCurrent(t) ? 'text-accent-light' : 'text-white'">
            {{ t.title || t.file_path.split(/[\\/]/).pop() }}
          </div>
        </div>
        <button
          class="shrink-0 px-1.5 py-0.5 rounded text-[10px] bg-bg-hover text-muted hover:text-white hover:bg-bg-border transition-colors"
          :title="'用本地播放器打开'"
          @click="openWithPlayer(t)"
        >
          <ExternalLink :size="11" /> 本地播放器
        </button>
        <span v-if="t.file_format" class="text-[9px] px-1 py-0.5 rounded bg-bg-hover text-muted uppercase">
          {{ t.file_format }}
        </span>
        <span class="text-[11px] text-muted tabular-nums shrink-0">
          {{ fmtDuration(t.duration_sec) }}
        </span>
      </div>
    </div>
  </div>
</template>
