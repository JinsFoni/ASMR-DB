<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Download,
  Pause,
  Play,
  X,
  Loader2,
  CheckCircle2,
  AlertCircle,
  Settings2,
} from "lucide-vue-next";
import { useDownloadStore } from "../stores/download";
import * as api from "../lib/api";
import EmptyState from "../components/common/EmptyState.vue";

const store = useDownloadStore();

const editingDir = ref(false);
const dirInput = ref("");

const fmtBytes = (b: number) => {
  if (!b) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = b;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
};

const statusMeta = computed(() => (s: string) => {
  switch (s) {
    case "downloading":
      return { label: "下载中", color: "text-accent-light" };
    case "done":
      return { label: "已完成", color: "text-green-400" };
    case "paused":
      return { label: "已暂停", color: "text-muted" };
    case "error":
    case "cancelled":
      return { label: s === "error" ? "失败" : "已取消", color: "text-accent" };
    default:
      return { label: "等待中", color: "text-muted" };
  }
});

onMounted(async () => {
  store.loadSettings();
  await store.refresh();
});

async function pickDir() {
  const dir = await open({ directory: true, multiple: false, title: "选择下载目录" });
  if (dir && typeof dir === "string") {
    dirInput.value = dir;
  }
}

async function saveSettings() {
  if (!dirInput.value) return;
  await store.saveSettings(dirInput.value, store.settings.concurrency);
  editingDir.value = false;
}

async function doPause(rj: string) {
  await api.pauseDownload(rj);
  await store.refresh();
}
async function doResume(rj: string) {
  await api.resumeDownload(rj);
  await store.refresh();
}
async function doCancel(rj: string) {
  await api.cancelDownload(rj);
  await store.refresh();
}
</script>

<template>
  <div class="p-5 max-w-3xl">
    <!-- 下载设置 -->
    <div class="card p-4 mb-4">
      <div class="flex items-center justify-between mb-2">
        <div class="text-xs font-semibold text-white flex items-center gap-1.5">
          <Settings2 :size="14" class="text-accent" /> 下载设置
        </div>
        <button v-if="!editingDir" class="btn-outline !text-[11px]" @click="editingDir = true">
          修改
        </button>
      </div>

      <div class="text-xs text-muted space-y-1">
        <div class="flex items-center gap-2">
          <span class="label w-16">下载目录</span>
          <template v-if="!editingDir">
            <span class="ellipsis-1 flex-1">{{ store.settings.download_dir || "未设置" }}</span>
          </template>
          <template v-else>
            <div class="flex flex-1 gap-2">
              <input v-model="dirInput" class="input !text-xs flex-1" placeholder="选择下载目录" />
              <button class="btn-outline !text-xs" @click="pickDir">浏览</button>
            </div>
          </template>
        </div>
        <div class="flex items-center gap-2">
          <span class="label w-16">并发数</span>
          <select v-model.number="store.settings.concurrency" class="input !py-1 !text-xs">
            <option :value="1">1</option>
            <option :value="2">2</option>
            <option :value="3">3</option>
            <option :value="4">4</option>
          </select>
        </div>
        <button v-if="editingDir" class="btn-primary !text-[11px] mt-1" @click="saveSettings">保存设置</button>
      </div>
    </div>

    <!-- 队列 -->
    <div class="text-sm font-semibold text-white mb-3">
      下载队列 ({{ store.tasks.length }})
    </div>

    <div v-if="!store.tasks.length" class="card">
      <EmptyState
        title="没有下载任务"
        desc="在作品详情页点击「下载」并填入下载链接，即可开始下载。"
        :icon="Download"
      />
    </div>

    <div v-else class="space-y-2">
      <div v-for="t in store.tasks" :key="t.rj_code" class="card p-3.5">
        <div class="flex items-center gap-3">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-xs font-medium text-white font-mono">{{ t.rj_code }}</span>
              <span class="text-[10px]" :class="statusMeta(t.status).color">
                {{ statusMeta(t.status).label }}
              </span>
              <span v-if="t.speed > 0" class="text-[10px] text-muted">{{ t.speed.toFixed(1) }} MB/s</span>
            </div>
            <div v-if="t.error" class="text-[11px] text-accent mt-0.5 ellipsis-1">{{ t.error }}</div>
          </div>

          <span class="text-[11px] text-muted tabular-nums shrink-0">
            {{ t.bytes_done ? fmtBytes(t.bytes_done) : "0 B" }} / {{ t.bytes_total ? fmtBytes(t.bytes_total) : "?" }}
          </span>

          <div class="flex items-center gap-1 shrink-0">
            <button
              v-if="t.status === 'downloading'"
              class="btn-ghost !p-1.5" title="暂停"
              @click="doPause(t.rj_code)"
            >
              <Pause :size="14" />
            </button>
            <button
              v-if="t.status === 'paused'"
              class="btn-ghost !p-1.5" title="继续"
              @click="doResume(t.rj_code)"
            >
              <Play :size="14" />
            </button>
            <button
              v-if="['downloading', 'paused'].includes(t.status)"
              class="btn-ghost !p-1.5 !text-accent" title="取消"
              @click="doCancel(t.rj_code)"
            >
              <X :size="14" />
            </button>
            <CheckCircle2 v-if="t.status === 'done'" :size="16" class="text-green-400" />
            <AlertCircle v-if="t.status === 'error'" :size="16" class="text-accent" />
            <Loader2 v-if="t.status === 'queued'" :size="16" class="text-muted animate-spin" />
          </div>
        </div>

        <!-- 进度条 -->
        <div class="h-1.5 rounded-full bg-bg-hover mt-2.5 overflow-hidden">
          <div
            class="h-full rounded-full transition-[width]"
            :class="t.status === 'done' ? 'bg-green-400' : 'bg-accent'"
            :style="{ width: Math.min(100, t.progress) + '%' }"
          />
        </div>
      </div>
    </div>
  </div>
</template>
