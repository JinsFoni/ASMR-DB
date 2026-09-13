<script setup lang="ts">
import { ref, watch } from "vue";
import { ChevronUp, Folder, FolderOpen, House, Loader2, X } from "lucide-vue-next";
import * as api from "../../lib/api";
import type { DirEntry } from "../../lib/api";

const props = defineProps<{
  visible: boolean;
  title?: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "select", path: string): void;
}>();

const loading = ref(false);
const error = ref<string | null>(null);
const currentPath = ref("");
const parentPath = ref<string | null>(null);
const entries = ref<DirEntry[]>([]);

watch(
  () => props.visible,
  (v) => {
    if (v) navigate("");
  }
);

async function navigate(path: string) {
  loading.value = true;
  error.value = null;
  try {
    const res = await api.listDir(path || undefined);
    currentPath.value = res.path;
    parentPath.value = res.parent;
    entries.value = res.entries.filter((e) => e.isDir);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function pick(path: string) {
  emit("select", path);
  emit("close");
}
</script>

<template>
  <div
    v-if="visible"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 animate-fade-in"
    @click.self="emit('close')"
  >
    <div class="card w-[560px] max-w-[92vw] max-h-[72vh] flex flex-col !p-4">
      <!-- 标题栏 -->
      <div class="flex items-center gap-2 mb-3">
        <FolderOpen :size="15" class="text-accent" />
        <span class="text-sm font-semibold text-white">{{ title || "选择文件夹" }}</span>
        <div class="flex-1" />
        <button class="btn-ghost !p-1.5" title="关闭" @click="emit('close')">
          <X :size="15" />
        </button>
      </div>

      <!-- 当前路径 + 导航 -->
      <div class="flex items-center gap-2 mb-2">
        <button
          class="btn-outline !text-xs !px-2 !py-1 flex items-center gap-1"
          :disabled="!parentPath || loading"
          title="上一级"
          @click="parentPath && navigate(parentPath)"
        >
          <ChevronUp :size="13" /> 上一级
        </button>
        <button
          class="btn-outline !text-xs !px-2 !py-1 flex items-center gap-1"
          :disabled="loading"
          title="回到主目录"
          @click="navigate('')"
        >
          <House :size="13" /> 主目录
        </button>
        <span class="text-[11px] text-muted ellipsis-1 flex-1 font-mono" :title="currentPath">
          {{ currentPath || "加载中..." }}
        </span>
      </div>

      <!-- 目录列表 -->
      <div class="flex-1 min-h-0 overflow-y-auto rounded-lg border border-bg-border bg-bg-deep">
        <div v-if="loading" class="flex items-center justify-center py-10 text-muted">
          <Loader2 :size="20" class="animate-spin" />
        </div>
        <div v-else-if="error" class="p-4 text-xs text-accent">{{ error }}</div>
        <div v-else-if="!entries.length" class="p-4 text-xs text-muted text-center">
          此目录下没有子文件夹
        </div>
        <button
          v-for="entry in entries"
          :key="entry.path"
          class="w-full flex items-center gap-2.5 px-3 py-2 text-left hover:bg-bg-hover transition-colors border-b border-bg-border/40 last:border-0"
          @click="navigate(entry.path)"
        >
          <Folder :size="15" class="text-accent/80 shrink-0" />
          <span class="text-xs text-white/90 ellipsis-1">{{ entry.name }}</span>
        </button>
      </div>

      <!-- 底部操作 -->
      <div class="flex items-center gap-2 mt-3">
        <span v-if="error" class="text-[11px] text-accent flex-1 ellipsis-1">{{ error }}</span>
        <div v-else class="flex-1" />
        <button class="btn-ghost !text-xs" @click="emit('close')">取消</button>
        <button class="btn-primary !text-xs" :disabled="!currentPath || loading" @click="pick(currentPath)">
          <FolderOpen :size="13" /> 选择当前目录
        </button>
      </div>
    </div>
  </div>
</template>
