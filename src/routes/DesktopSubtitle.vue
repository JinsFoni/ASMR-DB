<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { X } from "lucide-vue-next";
import { emitSubtitleClosed } from "../lib/desktopSubtitle";

const text = ref("");
let channel: BroadcastChannel | null = null;

onMounted(() => {
  channel = new BroadcastChannel("dlsite-asmr-subtitle");
  channel.onmessage = (e) => {
    if (e.data?.type === "text") text.value = e.data.text ?? "";
  };
});

onBeforeUnmount(() => {
  channel?.close();
  channel = null;
});

/** 关闭窗口：通知主窗口同步状态，然后关闭本弹窗。 */
function close() {
  emitSubtitleClosed();
  window.close();
}
</script>

<template>
  <div class="h-screen w-screen flex items-center justify-center overflow-hidden relative bg-black/80">
    <!-- 字幕文本 -->
    <p
      v-if="text"
      class="max-w-[85vw] px-6 py-3 rounded-2xl text-center text-white text-2xl md:text-3xl font-bold leading-relaxed whitespace-pre-line"
      style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9)"
    >
      {{ text }}
    </p>
    <span v-else class="text-white/60 text-3xl" style="text-shadow: 0 1px 6px rgba(0, 0, 0, 0.8)">
      ♫
    </span>

    <!-- 关闭按钮 -->
    <button
      class="absolute top-1 right-1 w-6 h-6 flex items-center justify-center rounded-md text-white/40 hover:text-white hover:bg-white/10 transition-colors"
      title="关闭悬浮歌词"
      @click.stop="close"
    >
      <X :size="14" />
    </button>
  </div>
</template>
