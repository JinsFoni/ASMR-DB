<script setup lang="ts">
import { onMounted, onBeforeUnmount, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { listen } from "@tauri-apps/api/event";
import Sidebar from "./components/layout/Sidebar.vue";
import TopBar from "./components/layout/TopBar.vue";
import MiniPlayer from "./components/layout/MiniPlayer.vue";
import { useWorksStore } from "./stores/works";
import { useDownloadStore } from "./stores/download";
import { usePlayerStore } from "./stores/player";
import * as api from "./lib/api";
import { applyTheme, getStoredTheme } from "./lib/theme";

const worksStore = useWorksStore();
const downloadStore = useDownloadStore();
const playerStore = usePlayerStore();
const route = useRoute();

// 立即从 local storage 应用主题防止闪烁
applyTheme(getStoredTheme());

let unlisten: (() => void) | null = null;

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
  // 桌面字幕窗口点关闭 → 关闭悬浮字幕
  listen<never>("desktop-subtitle:close", () => {
    playerStore.disableDesktopSubtitle();
  });
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  unlisten?.();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    class="h-screen flex flex-col overflow-hidden select-none"
    :class="route.name === 'desktop-subtitle' ? 'bg-transparent' : 'bg-bg text-white'"
  >
    <template v-if="route.name !== 'desktop-subtitle'">
      <div class="flex-1 flex min-h-0">
        <Sidebar v-if="route.name !== 'player'" />
        <div class="flex-1 flex flex-col min-w-0">
          <TopBar v-if="route.name !== 'player'" />
          <main class="flex-1 overflow-y-auto min-h-0">
            <router-view />
          </main>
        </div>
      </div>
      <MiniPlayer />
    </template>
    <router-view v-else />
  </div>
</template>
