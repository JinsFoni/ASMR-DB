<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { Check, CheckCircle2, Download, Loader2, ImageOff } from "lucide-vue-next";
import type { WorkView } from "../../lib/types";

const props = defineProps<{
  view: WorkView;
  selected?: boolean;
  selectMode?: boolean;
  /** poster: 3:4 竖版海报（默认，裁切）；cover: 完整横版封面（不裁切） */
  variant?: "poster" | "cover";
}>();
const emit = defineEmits<{ (e: "toggle-select", id: number): void }>();

const wide = computed(() => props.variant === "cover");

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
    <!-- 封面（海报模式高度由 WorksCover 注入的 --poster-img-h 控制，与横版卡片严格等高） -->
    <div
      class="relative bg-bg-hover overflow-hidden"
      :class="wide ? '' : 'h-[var(--poster-img-h,251px)]'"
    >
      <img
        v-if="view.work.cover_url"
        :src="view.work.cover_url"
        :alt="title"
        loading="lazy"
        class="group-hover:scale-105 transition-transform duration-300"
        :class="wide ? 'w-full h-auto' : 'w-full h-full object-cover'"
      />
      <div
        v-else
        class="w-full flex items-center justify-center text-muted/50"
        :class="wide ? 'aspect-[4/3]' : 'h-full'"
      >
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

      <!-- 标签（横版模式下移到信息区，避免遮挡封面） -->
      <div
        v-if="!wide && view.tags.length"
        class="absolute bottom-1.5 left-1.5 right-1.5 flex flex-wrap gap-1"
      >
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
      <div v-if="wide && view.tags.length" class="flex flex-wrap gap-1 mt-1.5">
        <span
          v-for="t in view.tags.slice(0, 4)"
          :key="t.id"
          class="px-1 py-0.5 rounded text-[9px] bg-bg-hover text-muted border border-bg-border"
        >
          {{ t.name }}
        </span>
      </div>
      <!-- 海报模式：发售日期行（与横版标签行同构等高，保持两种卡片信息区高度一致） -->
      <div v-else-if="!wide && view.work.sale_date" class="flex mt-1.5">
        <span class="px-1 py-0.5 rounded text-[9px] bg-bg-hover text-muted border border-bg-border">
          {{ view.work.sale_date.slice(0, 10) }} 发售
        </span>
      </div>
      <div class="flex items-center gap-2 mt-1.5 text-[10px] text-muted">
        <span v-if="view.track_count > 0">{{ view.track_count }} 音轨</span>
        <span v-if="view.work.duration_min">{{ view.work.duration_min }}分</span>
      </div>
    </div>
  </RouterLink>
</template>
