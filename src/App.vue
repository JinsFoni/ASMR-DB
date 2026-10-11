<script setup lang="ts">
import { onMounted, onBeforeUnmount, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import NavBar from "./components/layout/NavBar.vue";
import MiniPlayer from "./components/layout/MiniPlayer.vue";
import ToastHost from "./components/common/ToastHost.vue";
import { useWorksStore } from "./stores/works";
import { useDownloadStore } from "./stores/download";
import { usePlayerStore } from "./stores/player";
import * as api from "./lib/api";
import { onSubtitleClosed } from "./lib/desktopSubtitle";
import { applyTheme, getStoredTheme } from "./lib/theme";

const worksStore = useWorksStore();
const downloadStore = useDownloadStore();
const playerStore = usePlayerStore();
const route = useRoute();

// 立即从 local storage 应用主题防止闪烁
applyTheme(getStoredTheme());

let unlisten: (() => void) | null = null;
let unlistenSubtitleClose: (() => void) | null = null;

function onKeydown(e: KeyboardEvent) {
  const target = e.target as HTMLElement;
  if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
    return;
  }
  if (!playerStore.visible) return;
  switch (e.code) {
    case "Space":
      e.preventDefault();
      playerStore.togglePlay();
      break;
    case "ArrowRight":
      playerStore.seek(playerStore.currentTime + 5);
      break;
    case "ArrowLeft":
      playerStore.seek(playerStore.currentTime - 5);
      break;
    case "ArrowUp":
      e.preventDefault();
      playerStore.setVolume(playerStore.volume + 0.05);
      break;
    case "ArrowDown":
      e.preventDefault();
      playerStore.setVolume(playerStore.volume - 0.05);
      break;
  }
}

onMounted(async () => {
  // 从数据库读取最新设置并同步主题
  try {
    const s = await api.getSettings();
    if (s.theme) applyTheme(s.theme);
  } catch (_) {}

  await worksStore.loadTags();
  worksStore.loadGroups();
  worksStore.fetchWorks();
  downloadStore.loadSettings();
  downloadStore.refresh();
  unlisten = await api.onDownloadProgress((ev) => {
    downloadStore.handleProgress(ev);
  });
  // 悬浮歌词弹窗点关闭 → 关闭悬浮字幕
  unlistenSubtitleClose = onSubtitleClosed(() => {
    playerStore.disableDesktopSubtitle();
  });
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  unlisten?.();
  unlistenSubtitleClose?.();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<style scoped>
.app-main {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  /* 始终保留滚动条槽位：可滚动与不可滚动的页面间切换时，
     内容宽度不再突变 10px，避免居中的导航胶囊横移抖动 */
  scrollbar-gutter: stable;
  display: flex;
  flex-direction: column;
}
/* 导航与页面内容处于同一滚动流；页面自身用 flex:1 撑满剩余高度 */
.app-main > :deep(*) {
  flex-shrink: 0;
}
/* 纵向 flex 容器里 margin:auto 会让子项按内容收缩（max-width 失效表现为页面变窄），
   强制直接子元素拉伸到容器宽度，页面内部再用 max-width 居中 */
.app-main > :deep(*) {
  width: 100%;
}
</style>

<template>
  <div
    class="h-screen flex flex-col overflow-hidden select-none"
    :class="route.name === 'desktop-subtitle' ? 'bg-transparent' : 'text-white'"
  >
    <template v-if="route.name !== 'desktop-subtitle'">
      <!-- 页面滚动容器：列表页（ASMR ONE / DLsite）用 KeepAlive 缓存，从详情页返回时保留 tab、数据与滚动位置。
           导航放在滚动容器内部 sticky 悬浮，内容从毛玻璃胶囊下方滚过（与设计稿一致）；播放器页为全屏布局无导航 -->
      <main class="app-main">
        <NavBar v-if="route.name !== 'player'" />
        <router-view v-slot="{ Component }">
          <keep-alive :include="['AsmrOnlinePage', 'DlsitePage']">
            <component :is="Component" />
          </keep-alive>
        </router-view>
      </main>
      <MiniPlayer />
      <ToastHost />
    </template>
    <router-view v-else />
  </div>
</template>
