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
  Monitor,
  Loader2,
  ListMusic,
  ImageOff,
} from "lucide-vue-next";
import { usePlayerStore } from "../stores/player";

const router = useRouter();
const player = usePlayerStore();

const speeds = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const showSpeeds = ref(false);

const coverUrl = computed(() => player.work?.work.cover_url || "");
const trackTitle = computed(() => player.currentTrack?.title || "");
const workTitle = computed(
  () => player.work?.work.title_zh || player.work?.work.title_ja || player.work?.work.rj_code || ""
);

const progressPercent = computed(() => {
  if (player.duration <= 0) return 0;
  return (player.currentTime / player.duration) * 100;
});

const remainText = computed(() => {
  if (player.duration <= 0) return "-0:00";
  return `-${fmt(Math.max(0, player.duration - player.currentTime))}`;
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
  <div class="player-page">
    <!-- 模糊封面背景（设计稿 .ambience） -->
    <div
      v-if="coverUrl"
      class="ambience"
      :style="{ backgroundImage: `url(${coverUrl})` }"
    />
    <div class="ambience-tint" />

    <div class="pwrap">
      <div class="pleft">
        <!-- 顶栏：返回 + 字幕切换 -->
        <div class="ptop">
          <button class="back" title="返回" @click="router.back()">
            <ArrowLeft :size="16" />
          </button>
          <div class="sp" />
          <button
            class="icon-btn"
            :class="{ on: player.subtitleEnabled }"
            :title="player.subtitleEnabled ? '关闭字幕' : '开启字幕'"
            @click="player.toggleSubtitle()"
          >
            <Captions :size="17" />
          </button>
          <button
            class="icon-btn"
            :class="{ on: player.desktopSubtitleEnabled }"
            :title="player.desktopSubtitleEnabled ? '关闭桌面悬浮字幕' : '开启桌面悬浮字幕（类似桌面歌词）'"
            @click="player.toggleDesktopSubtitle()"
          >
            <Monitor :size="17" />
          </button>
        </div>

        <!-- 主舞台：封面 + 字幕 -->
        <div class="pstage">
          <div class="pnow">
            <div class="pcov">
              <img v-if="coverUrl" :src="coverUrl" />
              <div v-else class="w-full h-full flex items-center justify-center" style="color: var(--label-3)">
                <ImageOff :size="48" />
              </div>
            </div>

            <div class="nowhead">
              <div class="nt">
                <b :title="trackTitle">{{ trackTitle || "未播放" }}</b>
                <span :title="workTitle">{{ workTitle }}</span>
              </div>
            </div>

            <!-- 大进度条 -->
            <div class="bigprog" @click="onSeekClick">
              <div class="fill" :style="{ width: progressPercent + '%' }" />
              <div class="knob" :style="{ left: progressPercent + '%' }" />
            </div>
            <div class="time-row">
              <span>{{ fmt(player.currentTime) }}</span>
              <span>{{ remainText }}</span>
            </div>

            <!-- 主控制 -->
            <div class="bigctrls">
              <button class="bctrl" :title="'循环模式: ' + player.loopMode" @click="cycleLoop">
                <Repeat1 v-if="player.loopMode === 'one'" :size="20" style="color: var(--blue-deep)" />
                <Repeat v-else :size="20" :style="player.loopMode === 'all' ? 'color: var(--blue-deep)' : ''" />
              </button>
              <button class="bctrl" title="上一曲" @click="player.prev()">
                <SkipBack :size="27" />
              </button>
              <button class="bplay" :title="player.playing ? '暂停' : '播放'" @click="player.togglePlay()">
                <Loader2 v-if="player.buffering" :size="40" class="animate-spin" />
                <Pause v-else-if="player.playing" :size="40" />
                <Play v-else :size="40" />
              </button>
              <button class="bctrl" title="下一曲" @click="player.next()">
                <SkipForward :size="27" />
              </button>
              <div class="vol">
                <button class="mini-ico" @click="player.setVolume(player.volume > 0 ? 0 : 0.8)">
                  <VolumeX v-if="player.volume === 0" :size="17" />
                  <Volume2 v-else :size="17" />
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
            </div>

            <!-- 倍速 -->
            <div class="speed-row">
              <div class="relative">
                <button class="speedbtn" @click="showSpeeds = !showSpeeds">
                  {{ player.rate.toFixed(2).replace(/\.?0+$/, "") }}×
                </button>
                <div v-if="showSpeeds" class="speed-pop">
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
            </div>
          </div>

          <!-- 字幕区（设计稿 .cap-zone） -->
          <div class="cap-zone">
            <div
              v-if="player.subtitleEnabled && player.subtitleText"
              class="cap-line"
            >{{ player.subtitleText }}</div>
            <div v-else-if="player.subtitleEnabled" class="cap-status">
              {{ player.subtitleLoading ? "正在加载字幕…" : (player.subtitleCues.length ? "（等待字幕时间点）" : "未找到字幕文件 (.vtt / .srt / .lrc)") }}
            </div>
            <div v-else class="cap-status">字幕未开启 · 点击右上角按钮开启</div>
            <div v-if="player.subtitleEnabled && player.subtitleName" class="cap-status">
              已加载字幕 · {{ player.subtitleName }}
            </div>
          </div>
        </div>
      </div>

      <!-- 播放列表侧卡（设计稿 .plist） -->
      <aside class="plist">
        <div class="plist-h">
          <b>播放列表</b><sup v-if="player.playlist.length">{{ player.playlist.length }}</sup>
          <span class="sp" />
          <button class="hico" title="切换循环" @click="cycleLoop">
            <Repeat1 v-if="player.loopMode === 'one'" :size="15" style="color: var(--blue-deep)" />
            <Repeat v-else :size="15" />
          </button>
        </div>
        <div class="plist-items">
          <div
            v-for="(t, i) in player.playlist"
            :key="t.id"
            class="prow"
            :class="{ cur: i === player.currentIndex }"
            @click="player.playTrack(i)"
          >
            <div class="pth">
              <img v-if="coverUrl" :src="coverUrl" />
            </div>
            <div class="tx">
              <span class="nm">{{ t.title || t.file_path.split(/[\\/]/).pop() }}</span>
              <span class="ar">{{ workTitle }}</span>
            </div>
            <span class="du">{{ fmtShort(t.duration_sec || 0) }}</span>
          </div>
          <div v-if="!player.playlist.length" class="px-5 py-4 text-xs flex items-center gap-2" style="color: var(--label-3)">
            <ListMusic :size="14" /> 播放列表为空
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.player-page {
  position: relative;
  height: 100%;
  overflow: hidden;
  color: var(--label);
}
.ambience {
  position: absolute;
  inset: -90px;
  z-index: 0;
  pointer-events: none;
  background-size: cover;
  background-position: center 30%;
  filter: blur(110px) saturate(1.45) brightness(0.5);
}
.ambience-tint {
  position: absolute;
  inset: 0;
  z-index: 1;
  pointer-events: none;
  background:
    radial-gradient(70% 60% at 50% 46%, transparent 30%, rgba(8, 8, 10, 0.72) 100%),
    linear-gradient(rgba(8, 8, 10, 0.25), rgba(8, 8, 10, 0.45));
}

.pwrap {
  position: relative;
  z-index: 2;
  height: 100%;
  display: flex;
  min-width: 0;
}
.pleft {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.ptop {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 26px 0;
  flex-shrink: 0;
}
.ptop .sp {
  flex: 1;
}
.back {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--fill-1);
  color: var(--label-2);
  cursor: pointer;
  border: none;
  transition: background 0.2s;
}
.back:hover {
  background: var(--fill-2);
  color: var(--label);
}
.icon-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: var(--fill-1);
  color: var(--label-2);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s, color 0.2s;
  flex-shrink: 0;
}
.icon-btn:hover {
  background: var(--fill-2);
  color: var(--label);
}
.icon-btn.on {
  background: rgba(47, 143, 239, 0.2);
  color: var(--blue-deep);
}

.pstage {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 56px;
  padding: 0 44px;
  min-height: 0;
  overflow-y: auto;
}
.pnow {
  width: min(460px, 36vw);
  flex-shrink: 0;
  padding: 20px 0;
}
.pcov {
  width: 100%;
  aspect-ratio: 4 / 3;
  border-radius: 12px;
  overflow: hidden;
  background: var(--fill-1);
  box-shadow: var(--shadow-float), inset 0 0 0 0.5px rgba(255, 255, 255, 0.1);
}
.pcov img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.nowhead {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 20px;
}
.nt {
  flex: 1;
  min-width: 0;
}
.nt b {
  display: block;
  font-size: 19px;
  font-weight: 700;
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.nt span {
  display: block;
  font-size: 12.5px;
  color: var(--label-2);
  margin-top: 3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bigprog {
  height: 16px;
  display: flex;
  align-items: center;
  cursor: pointer;
  position: relative;
  margin-top: 10px;
}
.bigprog::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 4px;
  border-radius: 2px;
  background: var(--fill-2);
}
.bigprog .fill {
  position: absolute;
  left: 0;
  height: 4px;
  border-radius: 2px;
  background: #fff;
}
.bigprog .knob {
  position: absolute;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  transform: translateX(-50%);
}
.time-row {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--label-3);
  font-variant-numeric: tabular-nums;
  margin-top: 4px;
}

.bigctrls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 36px;
  margin-top: 16px;
}
.bctrl,
.bplay {
  background: none;
  border: none;
  color: var(--label);
  cursor: pointer;
  display: grid;
  place-items: center;
  padding: 6px;
  border-radius: 50%;
  transition: transform 0.15s;
}
.bctrl:hover,
.bplay:hover {
  transform: scale(1.1);
}
.vol {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--label-2);
}
.vol-range {
  width: 72px;
  accent-color: #d8d8dc;
  cursor: pointer;
}
.mini-ico {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  border: none;
  background: none;
  color: var(--label-2);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: background 0.15s, color 0.2s;
}
.mini-ico:hover {
  background: var(--fill-1);
  color: var(--label);
}

.speed-row {
  display: flex;
  justify-content: center;
  margin-top: 12px;
}
.speedbtn {
  height: 30px;
  padding: 0 13px;
  border: none;
  border-radius: 999px;
  background: var(--fill-1);
  color: var(--label-2);
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.speedbtn:hover {
  background: var(--fill-3);
  color: var(--label);
}
.speed-pop {
  position: absolute;
  bottom: 38px;
  left: 50%;
  transform: translateX(-50%);
  border-radius: var(--r-md);
  padding: 5px;
  z-index: 80;
  background: var(--glass);
  backdrop-filter: blur(24px) saturate(1.6);
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

/* 字幕区（设计稿 .cap-zone） */
.cap-zone {
  flex: 1;
  max-width: 560px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
  padding: 20px 0;
}
.cap-line {
  font-size: 21px;
  font-weight: 700;
  line-height: 1.6;
  letter-spacing: -0.01em;
  color: #fff;
}
.cap-status {
  font-size: 12px;
  color: var(--label-3);
}

/* 播放列表侧卡（设计稿 .plist） */
.plist {
  flex-shrink: 0;
  width: 372px;
  margin: 14px 14px 14px 0;
  padding: 16px 8px 10px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: rgba(20, 18, 24, 0.42);
  backdrop-filter: blur(40px) saturate(1.3);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.07);
}
.plist-h {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 12px 12px;
  flex-shrink: 0;
}
.plist-h b {
  font-size: 15px;
  font-weight: 700;
}
.plist-h sup {
  font-size: 10px;
  color: var(--label-3);
  align-self: flex-start;
  margin-top: 2px;
}
.plist-h .sp {
  flex: 1;
}
.hico {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: none;
  background: none;
  color: var(--label-2);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: background 0.15s, color 0.2s;
}
.hico:hover {
  background: var(--fill-1);
  color: var(--label);
}
.plist-items {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0 4px 4px;
}
.prow {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 7px 10px;
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.15s;
}
.prow:hover {
  background: var(--fill-1);
}
.prow.cur {
  background: var(--fill-2);
}
.pth {
  width: 44px;
  height: 44px;
  border-radius: 7px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--fill-1);
}
.pth img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.prow .tx {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.prow .nm {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.prow .ar {
  font-size: 11px;
  color: var(--label-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.prow .du {
  font-size: 11.5px;
  color: var(--label-2);
  font-variant-numeric: tabular-nums;
}

@media (max-width: 1100px) {
  .cap-zone {
    display: none;
  }
}
@media (max-width: 900px) {
  .pstage {
    flex-direction: column;
    gap: 18px;
  }
  .pnow {
    width: min(420px, 86vw);
  }
  .plist {
    display: none;
  }
}
</style>
