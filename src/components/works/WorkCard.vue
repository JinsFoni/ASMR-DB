<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { Check, CheckCircle2, Download, Loader2, ImageOff } from "lucide-vue-next";
import type { WorkView } from "../../lib/types";

const props = defineProps<{ view: WorkView; selected?: boolean; selectMode?: boolean }>();
const emit = defineEmits<{ (e: "toggle-select", id: number): void }>();

/** 选择模式下点击卡片切换选中，而非跳转详情 */
function onCardClick(e: MouseEvent) {
  if (props.selectMode) {
    e.preventDefault();
    emit("toggle-select", props.view.work.id);
  }
}

const statusMeta = computed(() => {
  switch (props.view.download_status) {
    case "downloaded":
      return { label: "已下载", cls: "bg-green-500/90 text-white", icon: CheckCircle2 };
    case "downloading":
      return { label: "下载中", cls: "bg-accent/90 text-white", icon: Loader2 };
    default:
      return { label: "", cls: "bg-black/60 text-white", icon: Download };
  }
});

const title = computed(
  () => props.view.work.title_zh || props.view.work.title_ja || props.view.work.rj_code
);
const circle = computed(() => props.view.work.circle_name || "");
</script>

<template>
  <RouterLink
    :to="`/work/${view.work.id}`"
    class="group block rounded-xl overflow-hidden border border-bg-border bg-bg-card hover:border-accent/50 hover:shadow-lg hover:shadow-accent/5 transition-all"
    :class="selectMode ? 'cursor-pointer' : ''"
    @click="onCardClick"
  >
    <!-- 封面 -->
    <div class="relative aspect-[3/4] bg-bg-hover overflow-hidden">
      <img
        v-if="view.work.cover_url"
        :src="view.work.cover_url"
        :alt="title"
        loading="lazy"
        class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
      />
      <div v-else class="w-full h-full flex items-center justify-center text-muted/50">
        <ImageOff :size="28" />
      </div>

      <!-- 选择模式复选框 -->
      <div
        v-if="selectMode"
        class="absolute top-1.5 left-1.5 z-10 w-5 h-5 rounded-md border-2 flex items-center justify-center transition-colors"
        :class="selected ? 'bg-accent border-accent text-white' : 'bg-black/50 border-white/60 text-white'"
      >
        <Check v-if="selected" :size="14" />
      </div>

      <!-- 状态角标 -->
      <div
        v-if="statusMeta.label && !selectMode"
        class="absolute top-1.5 right-1.5 px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 backdrop-blur"
        :class="statusMeta.cls"
      >
        <component :is="statusMeta.icon" :size="10" />
        {{ statusMeta.label }}
      </div>

      <!-- 标签 -->
      <div v-if="view.tags.length" class="absolute bottom-1.5 left-1.5 right-1.5 flex flex-wrap gap-1">
        <span
          v-for="t in view.tags.slice(0, 3)"
          :key="t.id"
          class="px-1 py-0.5 rounded text-[9px] bg-black/60 text-white/90"
        >
          {{ t.name }}
        </span>
      </div>
    </div>

    <!-- 信息 -->
    <div class="p-2.5">
      <div class="text-xs font-medium text-white ellipsis-2 min-h-[2.2rem]" :title="title">
        {{ title }}
      </div>
      <div class="text-[10px] text-muted mt-1 ellipsis-1">
        <span class="font-mono">{{ view.work.rj_code }}</span>
        <span v-if="circle"> · {{ circle }}</span>
      </div>
      <div class="flex items-center gap-2 mt-1.5 text-[10px] text-muted">
        <span v-if="view.track_count > 0">{{ view.track_count }} 音轨</span>
        <span v-if="view.work.duration_min">{{ view.work.duration_min }}分</span>
      </div>
    </div>
  </RouterLink>
</template>
