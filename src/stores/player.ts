import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { Howl } from "howler";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track, WorkView } from "../lib/types";
import * as api from "../lib/api";
import { detectAndParse, getActiveCue } from "../lib/subtitle";
import {
  openDesktopSubtitleWindow,
  closeDesktopSubtitleWindow,
  pushDesktopSubtitleText,
} from "../lib/desktopSubtitle";

export type LoopMode = "none" | "one" | "all";

export const usePlayerStore = defineStore("player", () => {
  // Current context
  const work = ref<WorkView | null>(null);
  const playlist = ref<Track[]>([]);
  const currentIndex = ref(0);

  // Playback state
  const playing = ref(false);
  const currentTime = ref(0);
  const duration = ref(0);
  const volume = ref(0.8);
  const rate = ref(1.0);
  const loopMode = ref<LoopMode>("all");
  const buffering = ref(false);

  // 字幕状态
  const subtitleEnabled = ref(false); // 默认不显示
  const subtitleCues = ref<{ start: number; end: number; text: string }[]>([]);
  const subtitleName = ref<string | null>(null);
  const subtitleLoading = ref(false);
  let subtitleReqSeq = 0;

  // 桌面悬浮字幕
  const desktopSubtitleEnabled = ref(false);
  let desktopSubtitleTimer: number | null = null;

  let howl: Howl | null = null;
  let seekTimer: number | null = null;
  let progressTimer: number | null = null;
  let consecutiveErrors = 0;

  const currentTrack = computed<Track | null>(
    () => playlist.value[currentIndex.value] ?? null
  );

  const visible = computed(() => !!work.value);

  function stopHowl() {
    if (howl) {
      howl.stop();
      howl.unload();
      howl = null;
    }
    if (seekTimer) {
      window.clearInterval(seekTimer);
      seekTimer = null;
    }
    if (progressTimer) {
      window.clearInterval(progressTimer);
      progressTimer = null;
    }
    playing.value = false;
    currentTime.value = 0;
    duration.value = 0;
  }

  function persistProgress() {
    const w = work.value;
    if (!w || !howl) return;
    const track = currentTrack.value;
    // 在线播放（asmr.one）不写进度，避免 track_id 与本地音轨混淆
    if (track?.file_path.startsWith("http")) return;
    api.savePlayProgress(w.work.id, track?.id ?? null, howl.seek() as number).catch(
      () => {}
    );
  }

  function load(tracks: Track[], startIndex = 0, initialTime = 0) {
    stopHowl();
    playlist.value = tracks;
    currentIndex.value = startIndex;
    playIndex(startIndex, initialTime);
  }

  function playIndex(index: number, initialTime = 0) {
    const track = playlist.value[index];
    if (!track) return;
    currentIndex.value = index;
    stopHowl();
    if (!track.file_path) return;

    buffering.value = true;
    // 注意：不在这里重置 consecutiveErrors，否则 onloaderror 里跳下一首会重置计数，
    // 导致全部音轨加载失败时无限循环。改为在 onplay（真正开始播放）时重置。
    // 在线 URL（asmr.one 自定义协议）直接用，本地路径才走 convertFileSrc
    const src = track.file_path.startsWith("http")
      ? track.file_path
      : convertFileSrc(track.file_path);
    howl = new Howl({
      src: [src],
      html5: true,
      volume: volume.value,
      rate: rate.value,
      onplay: () => {
        playing.value = true;
        buffering.value = false;
        consecutiveErrors = 0;
        duration.value = howl?.duration() ?? 0;
        startTimers();
      },
      onpause: () => {
        playing.value = false;
        stopTimers();
        persistProgress();
      },
      onstop: () => {
        playing.value = false;
        stopTimers();
      },
      onend: () => {
        playing.value = false;
        stopTimers();
        persistProgress();
        handleTrackEnd();
      },
      onloaderror: () => {
        buffering.value = false;
        consecutiveErrors++;
        if (consecutiveErrors >= playlist.value.length) {
          // 全部音轨都加载失败，停止尝试（避免循环）
          playing.value = false;
          currentTime.value = 0;
          return;
        }
        // 尝试下一首（跳过无法播放的文件）
        handleTrackEnd();
      },
    });
    if (initialTime > 0) howl.seek(initialTime);
    howl.play();
    // 自动查找字幕文件（lrc / vtt / srt，支持本地与在线试听）
    loadSubtitle(track);
  }

  function handleTrackEnd() {
    if (loopMode.value === "one") {
      playIndex(currentIndex.value);
    } else if (currentIndex.value < playlist.value.length - 1) {
      playIndex(currentIndex.value + 1);
    } else if (loopMode.value === "all") {
      playIndex(0);
    }
    // none → stop at end
  }

  function startTimers() {
    stopTimers();
    progressTimer = window.setInterval(() => {
      if (howl) {
        currentTime.value = howl.seek() as number;
      }
    }, 500);
  }

  function stopTimers() {
    if (progressTimer) {
      window.clearInterval(progressTimer);
      progressTimer = null;
    }
    if (seekTimer) {
      window.clearInterval(seekTimer);
      seekTimer = null;
    }
  }

  function togglePlay() {
    if (!howl) return;
    if (playing.value) {
      howl.pause();
    } else {
      howl.play();
    }
  }

  function seek(sec: number) {
    if (!howl) return;
    howl.seek(Math.max(0, sec));
    currentTime.value = sec;
  }

  function setVolume(v: number) {
    volume.value = Math.max(0, Math.min(1, v));
    if (howl) howl.volume(volume.value);
  }

  function setRate(r: number) {
    rate.value = r;
    if (howl) howl.rate(r);
  }

  function next() {
    if (currentIndex.value < playlist.value.length - 1) {
      playIndex(currentIndex.value + 1);
    }
  }

  function prev() {
    if (currentTime.value > 3) {
      seek(0);
    } else if (currentIndex.value > 0) {
      playIndex(currentIndex.value - 1);
    } else {
      seek(0);
    }
  }

  function playTrack(index: number) {
    playIndex(index);
  }

  /** 自动查找并解析当前音轨的字幕文件（.lrc / .vtt / .srt，支持本地与 asmr.one 在线试听）。 */
  async function loadSubtitle(track: Track) {
    const seq = ++subtitleReqSeq;
    subtitleLoading.value = true;
    try {
      if (track.file_path.startsWith("http")) {
        // 在线试听模式（asmr.one）
        const subs = track.subtitles || [];
        if (!subs.length) {
          subtitleCues.value = [];
          subtitleName.value = null;
          return;
        }

        // 智能挑选最佳字幕：
        // 1. 中文（zh / chs / cht / cmn / chi / 中 / 简 / 繁 / 译）
        // 2. 日文（ja / jpn / 日）
        // 3. 英文（en / eng）
        // 4. 首个可用字幕
        const chosen =
          subs.find((s) => /zh|chs|cht|cmn|chi|中|简|繁|译/i.test(s.title)) ||
          subs.find((s) => /ja|jpn|日/i.test(s.title)) ||
          subs.find((s) => /en|eng/i.test(s.title)) ||
          subs[0];

        if (!chosen || !chosen.url) {
          subtitleCues.value = [];
          subtitleName.value = null;
          return;
        }

        const content = await api.fetchSubtitleContent(chosen.url);
        if (seq !== subtitleReqSeq) return;

        if (content && content.trim()) {
          subtitleCues.value = detectAndParse(content);
          subtitleName.value = chosen.title;
        } else {
          subtitleCues.value = [];
          subtitleName.value = null;
        }
      } else {
        // 本地文件模式
        const res = await api.findSubtitleForTrack(track.file_path);
        if (seq !== subtitleReqSeq) return;
        if (res) {
          subtitleCues.value = detectAndParse(res.content);
          subtitleName.value = res.name;
        } else {
          subtitleCues.value = [];
          subtitleName.value = null;
        }
      }
    } catch (e) {
      console.warn("加载字幕失败:", e);
      if (seq === subtitleReqSeq) {
        subtitleCues.value = [];
        subtitleName.value = null;
      }
    } finally {
      if (seq === subtitleReqSeq) subtitleLoading.value = false;
    }
  }

  function toggleSubtitle() {
    subtitleEnabled.value = !subtitleEnabled.value;
  }

  // ---------- 桌面悬浮字幕 ----------

  function startDesktopSubtitleTimer() {
    stopDesktopSubtitleTimer();
    // 开启后立即推一次
    pushDesktopSubtitleText(subtitleText.value);
    desktopSubtitleTimer = window.setInterval(() => {
      pushDesktopSubtitleText(subtitleText.value);
    }, 300);
  }

  function stopDesktopSubtitleTimer() {
    if (desktopSubtitleTimer) {
      window.clearInterval(desktopSubtitleTimer);
      desktopSubtitleTimer = null;
    }
  }

  async function enableDesktopSubtitle() {
    if (desktopSubtitleEnabled.value) return;
    desktopSubtitleEnabled.value = true;
    await openDesktopSubtitleWindow();
    startDesktopSubtitleTimer();
  }

  async function disableDesktopSubtitle() {
    if (!desktopSubtitleEnabled.value) return;
    desktopSubtitleEnabled.value = false;
    stopDesktopSubtitleTimer();
    await closeDesktopSubtitleWindow();
  }

  function toggleDesktopSubtitle() {
    if (desktopSubtitleEnabled.value) disableDesktopSubtitle();
    else enableDesktopSubtitle();
  }

  /** 当前播放时间对应的字幕行文本。 */
  const subtitleText = computed(() =>
    getActiveCue(subtitleCues.value, currentTime.value)?.text ?? ""
  );

  /** 从某作品的音轨列表加载并播放指定音轨 */
  function playFromWork(w: WorkView, tracks: Track[], index = 0, initialTime = 0) {
    work.value = w;
    load(tracks, index, initialTime);
  }

  return {
    work,
    playlist,
    currentIndex,
    playing,
    currentTime,
    duration,
    volume,
    rate,
    loopMode,
    buffering,
    currentTrack,
    visible,
    subtitleEnabled,
    subtitleCues,
    subtitleName,
    subtitleLoading,
    subtitleText,
    desktopSubtitleEnabled,
    loadSubtitle,
    toggleSubtitle,
    enableDesktopSubtitle,
    disableDesktopSubtitle,
    toggleDesktopSubtitle,
    load,
    playIndex,
    togglePlay,
    seek,
    setVolume,
    setRate,
    next,
    prev,
    playTrack,
    playFromWork,
  };
});
