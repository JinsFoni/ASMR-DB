<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import {
  Loader2,
  RefreshCw,
  Languages,
  RotateCcw,
  Trash2,
  CheckCircle2,
  XCircle,
} from "lucide-vue-next";
import * as api from "../lib/api";
import type { TranslationTask } from "../lib/types";
import Pagination from "../components/common/Pagination.vue";

const router = useRouter();

const items = ref<TranslationTask[]>([]);
const page = ref(1);
const totalPages = ref(1);
const total = ref(0);
const loading = ref(false);
const error = ref("");
const busyId = ref<number | null>(null);
const batchLoading = ref(false);
const batchMsg = ref("");

let pollTimer: number | null = null;

const hasActive = computed(() =>
  items.value.some((t) => t.status === "pending" || t.status === "processing")
);

const statusMeta = computed(() => (s: string) => {
  switch (s) {
    case "success":
      return { label: "成功", cls: "text-green-400", icon: CheckCircle2 };
    case "failed":
      return { label: "失败", cls: "text-accent", icon: XCircle };
    case "processing":
      return { label: "进行中", cls: "text-accent-light", icon: Loader2 };
    default:
      return { label: "排队中", cls: "text-muted", icon: null };
  }
});

async function load() {
  loading.value = !pollTimer;
  error.value = "";
  try {
    const res = await api.listTranslations(page.value);
    items.value = res.items;
    totalPages.value = res.totalPages;
    total.value = res.total;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function goPage(p: number) {
  page.value = p;
  load();
}

async function batchTranslate() {
  if (batchLoading.value) return;
  if (!confirm("将所有未翻译的作品加入翻译队列？")) return;
  batchLoading.value = true;
  batchMsg.value = "";
  try {
    const res = await api.enqueueMissingTranslations();
    batchMsg.value = res.queued > 0 ? `已加入 ${res.queued} 部作品到翻译队列` : "没有需要翻译的作品";
    await load();
  } catch (e) {
    batchMsg.value = `失败：${e}`;
  } finally {
    batchLoading.value = false;
  }
}

async function retry(t: TranslationTask) {
  if (busyId.value) return;
  busyId.value = t.id;
  try {
    await api.retryTranslation(t.id);
    await load();
  } catch (e) {
    alert(`重试失败: ${e}`);
  } finally {
    busyId.value = null;
  }
}

async function remove(t: TranslationTask) {
  if (busyId.value) return;
  if (!confirm(`删除「${t.title || t.rj_code || t.id}」的翻译记录？（已写入作品的中文标题/简介会保留）`)) return;
  busyId.value = t.id;
  try {
    await api.deleteTranslation(t.id);
    await load();
  } catch (e) {
    alert(`删除失败: ${e}`);
  } finally {
    busyId.value = null;
  }
}

function openWork(t: TranslationTask) {
  if (t.status === "success" && t.work_id) router.push(`/work/${t.work_id}`);
}

onMounted(() => {
  load();
  // 有排队/进行中任务时轮询刷新状态
  pollTimer = window.setInterval(() => {
    if (hasActive.value && !loading.value) load();
  }, 3000);
});

onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer);
});
</script>

<template>
  <div class="p-5 space-y-4">
    <!-- 工具栏 -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="text-xs text-muted flex items-center gap-1.5">
        <Languages :size="13" class="text-accent-light" />
        日文标题/简介 → 简体中文 · 共 {{ total }} 条记录
      </div>
      <div class="flex items-center gap-2">
        <button class="btn-primary !text-xs" :disabled="batchLoading" @click="batchTranslate">
          <Loader2 v-if="batchLoading" :size="13" class="animate-spin" />
          <Languages v-else :size="13" />
          翻译全部未翻译作品
        </button>
        <button class="btn-outline !text-xs !py-1.5" title="刷新" @click="load">
          <RefreshCw :size="13" />
        </button>
      </div>
    </div>
    <p v-if="batchMsg" class="text-xs" :class="batchMsg.startsWith('失败') ? 'text-accent' : 'text-green-400'">
      {{ batchMsg }}
    </p>

    <!-- 错误 / 加载 / 空 -->
    <div v-if="error" class="card p-4 text-xs text-muted">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div v-else-if="!items.length" class="text-center py-20 text-muted text-sm">
      暂无翻译记录。开启「入库自动翻译」或在上方批量翻译未翻译作品。
    </div>

    <!-- 记录列表 -->
    <div v-else class="space-y-2">
      <div
        v-for="t in items"
        :key="t.id"
        class="card p-3 flex items-start gap-3 transition-colors"
        :class="t.status === 'success' ? 'cursor-pointer hover:border-accent/50' : ''"
        :title="t.status === 'success' ? '点击查看作品' : undefined"
        @click="t.status === 'success' && openWork(t)"
      >
        <!-- 状态 -->
        <span
          class="shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium flex items-center gap-1 w-18 justify-center"
          :class="statusMeta(t.status).cls"
        >
          <Loader2 v-if="t.status === 'processing'" :size="10" class="animate-spin" />
          <component
            v-else-if="statusMeta(t.status).icon"
            :is="statusMeta(t.status).icon"
            :size="10"
          />
          {{ statusMeta(t.status).label }}
        </span>

        <!-- 内容 -->
        <div class="flex-1 min-w-0">
          <div class="text-xs font-medium" :class="t.status === 'success' ? 'text-accent-light' : 'text-white'">
            {{ t.title || t.rj_code || `#${t.work_id}` }}
          </div>
          <!-- 标题译文 -->
          <div v-if="t.translated_title" class="text-xs text-white/80 mt-1 ellipsis-1">
            <Languages :size="10" class="inline mr-1 text-muted" />{{ t.translated_title }}
          </div>
          <div v-if="t.source_title && t.status !== 'success'" class="text-[11px] text-muted mt-1 ellipsis-1">
            原文：{{ t.source_title }}
          </div>
          <div v-if="t.translated_desc" class="text-[11px] text-muted mt-0.5 ellipsis-1">
            简介已翻译（{{ t.translated_desc.length }} 字）
          </div>
          <div v-if="t.error" class="text-[11px] text-accent mt-1 ellipsis-1" :title="t.error">
            ⚠️ {{ t.error }}
          </div>
          <div class="text-[10px] text-muted/70 mt-1">
            <span v-if="t.rj_code" class="font-mono">{{ t.rj_code }}</span>
            <span v-if="t.updated_at"> · {{ t.updated_at.replace("T", " ").slice(0, 16) }}</span>
          </div>
        </div>

        <!-- 操作 -->
        <div class="flex items-center gap-1 shrink-0" @click.stop>
          <button
            class="btn-ghost !p-1.5 !text-muted hover:!text-white"
            :title="t.status === 'success' ? '重新翻译' : '重试'"
            :disabled="busyId === t.id || t.status === 'processing' || t.status === 'pending'"
            @click="retry(t)"
          >
            <RotateCcw :size="13" />
          </button>
          <button
            class="btn-ghost !p-1.5 !text-muted hover:!text-accent"
            title="删除记录"
            :disabled="busyId === t.id"
            @click="remove(t)"
          >
            <Trash2 :size="13" />
          </button>
        </div>
      </div>
    </div>

    <!-- 分页 -->
    <Pagination
      v-if="totalPages > 1"
      :page="page"
      :total="totalPages"
      @change="goPage"
    />
  </div>
</template>
