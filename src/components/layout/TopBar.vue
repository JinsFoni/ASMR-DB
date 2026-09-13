<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useDownloadStore } from "../../stores/download";
import { Download, Play, RefreshCw } from "lucide-vue-next";
import { usePlayerStore } from "../../stores/player";

const route = useRoute();
const downloadStore = useDownloadStore();
const playerStore = usePlayerStore();

const title = computed(() => {
  switch (route.name) {
    case "home":
      return "📚 作品库";
    case "import":
      return "➕ 导入作品";
    case "downloads":
      return "⬇️ 下载管理";
    case "settings":
      return "⚙️ 设置";
    case "work-detail":
      return "📄 作品详情";
    default:
      return "DLsite Manager";
  }
});

function reloadPage() {
  window.location.reload();
}
</script>

<template>
  <header class="h-12 shrink-0 flex items-center justify-between px-4 border-b border-bg-border bg-bg-card/70 backdrop-blur">
    <h1 class="text-sm font-semibold text-white">{{ title }}</h1>
    <div class="flex items-center gap-2">
      <button
        v-if="playerStore.visible"
        class="btn-ghost text-xs"
        title="跳转到播放器"
        @click="$router.push({ name: 'home' })"
      >
        <Play :size="14" />
        {{ playerStore.playing ? "播放中" : "已暂停" }}
      </button>
      <button
        class="btn-ghost !p-1.5"
        title="重新加载页面"
        @click="reloadPage"
      >
        <RefreshCw :size="14" />
      </button>
      <button
        class="btn-ghost text-xs relative"
        title="下载任务"
        @click="$router.push({ name: 'downloads' })"
      >
        <Download :size="15" />
        下载
        <span
          v-if="downloadStore.activeCount > 0"
          class="absolute -top-1 -right-1 min-w-4 h-4 px-1 rounded-full bg-accent text-white text-[10px] flex items-center justify-center"
        >
          {{ downloadStore.activeCount }}
        </span>
      </button>
    </div>
  </header>
</template>
