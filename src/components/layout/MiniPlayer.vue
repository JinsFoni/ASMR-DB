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
  ImageOff,
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
    class="player-dock glass"
  >
    <!-- 曲目 -->
    <div class="np">
      <div class="np-art overflow-hidden">
        <img
          v-if="player.work?.work.cover_url"
          :src="player.work.work.cover_url"
          class="w-full h-full object-cover"
          alt=""
        />
        <ImageOff v-else :size="18" class="text-white/60" />
      </div>
      <div class="min-w-0">
        <p class="np-title">{{ player.currentTrack?.title || "未播放" }}</p>
        <p class="np-sub ellipsis-1">
          {{ player.work?.work.title_zh || player.work?.work.title_ja || player.work?.work.rj_code || "" }}
        </p>
      </div>
    </div>

    <!-- 控制 + 进度 -->
    <div class="pcenter">
      <div class="pctrls">
        <button class="pctrl" title="上一首" @click="player.prev()">
          <SkipBack :size="17" />
        </button>
        <button class="pmain" :title="player.playing ? '暂停' : '播放'" @click="player.togglePlay()">
          <Loader2 v-if="player.buffering" :size="15" class="animate-spin" />
          <Pause v-else-if="player.playing" :size="15" />
          <Play v-else :size="15" class="ml-0.5" />
        </button>
        <button class="pctrl" title="下一首" @click="player.next()">
          <SkipForward :size="17" />
        </button>
      </div>
      <div class="pprog">
        <time>{{ fmt(player.currentTime) }}</time>
        <div class="pbar" @click="onSeekClick">
          <i :style="{ width: progressPercent + '%' }" />
        </div>
        <time>{{ fmt(player.duration) }}</time>
      </div>
    </div>

    <!-- 右侧：倍速 / 循环 / 音量 / 展开 -->
    <div class="pright">
      <div class="relative">
        <button class="pspeed" @click="showSpeeds = !showSpeeds">
          {{ player.rate.toFixed(2).replace(/\.?0+$/, "") }}×
        </button>
        <div v-if="showSpeeds" class="speed-pop glass">
          <button
            v-for="s in speeds"
            :key="s"
            class="speed-item"
            :class="{ on: player.rate === s }"
            @click="player.setRate(s); showSpeeds = false"
          >
            {{ s.toFixed(2).replace(/\.?0+$/, "") }}×
          </button>
        </div>
      </div>

      <button
        class="pctrl"
        :title="'循环模式: ' + player.loopMode"
        @click="cycleLoop"
      >
        <Repeat1 v-if="player.loopMode === 'one'" :size="16" class="text-accent-light" />
        <Repeat v-else :size="16" :class="player.loopMode === 'all' ? 'text-accent-light' : ''" />
      </button>

      <div class="pvol">
        <button class="pctrl" @click="player.setVolume(player.volume > 0 ? 0 : 0.8)">
          <VolumeX v-if="player.volume === 0" :size="15" />
          <Volume2 v-else :size="15" />
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="player.volume"
          class="vol-range"
          @input="player.setVolume(parseFloat(($event.target as HTMLInputElement).value))"
        />
      </div>

      <button class="pctrl" title="全屏播放器（字幕）" @click="router.push({ name: 'player' })">
        <Expand :size="16" />
      </button>
    </div>
  </footer>
</template>

<style scoped>
.player-dock {
  position: fixed;
  left: 50%;
  bottom: 18px;
  z-index: 70;
  transform: translateX(-50%);
  width: min(720px, calc(100% - 40px));
  backdrop-filter: blur(28px) saturate(1.7);
  -webkit-backdrop-filter: blur(28px) saturate(1.7);
  border-radius: 24px;
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.8),
    inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
  padding: 12px 20px;
  display: grid;
  grid-template-columns: minmax(160px, 200px) 1fr minmax(150px, auto);
  align-items: center;
  gap: 18px;
}
html[data-theme="light"] .player-dock,
html[data-theme="sakura"] .player-dock {
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.25),
    inset 0 0 0 0.5px rgba(0, 0, 0, 0.06);
}

.np {
  display: flex;
  align-items: center;
  gap: 13px;
  min-width: 0;
}
.np-art {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: var(--fill-2);
  flex-shrink: 0;
  box-shadow: 0 4px 12px -4px rgba(0, 0, 0, 0.35);
  display: grid;
  place-items: center;
}
.np-title {
  font-size: 13.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.np-sub {
  font-size: 11.5px;
  color: var(--label-2);
  margin-top: 1px;
}

.pcenter {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}
.pctrls {
  display: flex;
  align-items: center;
  gap: 20px;
}
.pctrl {
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
.pctrl:hover {
  opacity: 1;
  transform: scale(1.08);
}
.pmain {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  background: #fff;
  color: #0d0d0f;
  border: none;
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: transform 0.15s, background 0.15s;
}
.pmain:hover {
  transform: scale(1.07);
  background: var(--blue);
  color: #fff;
}

.pprog {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  max-width: 520px;
}
.pprog time {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--label-2);
  font-variant-numeric: tabular-nums;
}
.pbar {
  flex: 1;
  height: 14px;
  display: flex;
  align-items: center;
  cursor: pointer;
  position: relative;
}
.pbar::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 4px;
  border-radius: 2px;
  background: var(--fill-2);
}
.pbar i {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: 2px;
  background: var(--label);
}
.pbar i::after {
  content: "";
  position: absolute;
  right: -6px;
  top: 50%;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 5px rgba(0, 0, 0, 0.5), 0 0 0 0.5px rgba(255, 255, 255, 0.2);
  transform: translateY(-50%);
}

.pright {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 14px;
}
.pspeed {
  font-size: 12px;
  font-weight: 600;
  color: var(--label-2);
  background: var(--fill-1);
  border: none;
  border-radius: 999px;
  height: 30px;
  padding: 0 13px;
  cursor: pointer;
  transition: background 0.15s;
}
.pspeed:hover {
  background: var(--fill-3);
  color: var(--label);
}
.speed-pop {
  position: absolute;
  bottom: 38px;
  right: 0;
  border-radius: var(--r-md);
  padding: 5px;
  z-index: 80;
  box-shadow: var(--shadow-float), inset 0 0 0 0.5px rgba(255, 255, 255, 0.14);
  display: flex;
  flex-direction: column;
  min-width: 76px;
}
.speed-item {
  border: none;
  background: none;
  text-align: left;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--label-2);
  padding: 6px 12px;
  border-radius: 8px;
  cursor: pointer;
}
.speed-item:hover {
  background: var(--fill-1);
  color: var(--label);
}
.speed-item.on {
  color: var(--blue);
  font-weight: 700;
}

.pvol {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--label-2);
}
.vol-range {
  width: 70px;
  accent-color: var(--label);
  cursor: pointer;
}

@media (max-width: 900px) {
  .player-dock {
    grid-template-columns: 1fr;
    gap: 8px;
    padding: 12px 16px;
    bottom: 12px;
  }
  .pright {
    display: none;
  }
  .np {
    padding-bottom: 2px;
  }
}
</style>
