<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-vue-next";

const text = ref("");
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  unlisten = await listen<{ text: string }>("desktop-subtitle:text", (e) => {
    text.value = e.payload.text;
  });
});

onBeforeUnmount(() => {
  unlisten?.();
});

/** 关闭窗口：直接关闭本窗口（最可靠），同时通知主窗口同步状态。 */
async function close() {
  emitTo("main", "desktop-subtitle:close").catch(() => {});
  try {
    await getCurrentWindow().close();
  } catch {
    // ignore
  }
}
</script>

<template>
  <div
    class="h-screen w-screen flex items-center justify-center overflow-hidden relative"
    data-tauri-drag-region
  >
    <!-- 字幕文本 -->
    <p
      v-if="text"
      class="max-w-[85vw] px-6 py-3 rounded-2xl text-center text-white text-2xl md:text-3xl font-bold leading-relaxed whitespace-pre-line"
      style="
        background: rgba(0, 0, 0, 0.4);
        backdrop-filter: blur(6px);
        text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);
      "
    >
      {{ text }}
    </p>
    <span v-else class="text-white/60 text-3xl" style="text-shadow: 0 1px 6px rgba(0, 0, 0, 0.8)">
      ♫
    </span>

    <!-- 关闭按钮 -->
    <button
      class="absolute top-1 right-1 w-6 h-6 flex items-center justify-center rounded-md text-white/40 hover:text-white hover:bg-white/10 transition-colors"
      title="关闭桌面字幕"
      @click.stop="close"
    >
      <X :size="14" />
    </button>
  </div>
</template>
