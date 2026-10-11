<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import { Loader2, RefreshCw, Languages, RotateCcw, Trash2 } from "lucide-vue-next";
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

const statusBadge = computed(() => (s: string) => {
  switch (s) {
    case "success":
      return { label: "成功", cls: "green" };
    case "failed":
      return { label: "失败", cls: "red" };
    case "processing":
      return { label: "进行中", cls: "blue" };
    default:
      return { label: "排队中", cls: "gray" };
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
  <div class="page-wrap">
    <div class="page-head">
      <div>
        <h1>LLM 翻译</h1>
        <p class="sub">日文标题/简介 → 简体中文 · 共 {{ total }} 条记录</p>
      </div>
      <div class="flex items-center gap-2.5">
        <button class="btn primary" :disabled="batchLoading" @click="batchTranslate">
          <Loader2 v-if="batchLoading" :size="14" class="animate-spin" />
          <Languages v-else :size="14" />
          翻译全部未翻译作品
        </button>
        <button class="btn icon-only" title="刷新" style="width: 38px; padding: 0" @click="load">
          <RefreshCw v-if="!loading" :size="14" />
          <Loader2 v-else :size="14" class="animate-spin" />
        </button>
      </div>
    </div>

    <p v-if="batchMsg" class="text-xs" :style="{ color: batchMsg.startsWith('失败') ? 'var(--red)' : 'var(--green)' }">
      {{ batchMsg }}
    </p>

    <!-- 错误 / 加载 / 空 -->
    <div v-if="error" class="card p-4 text-xs" style="color: var(--label-2)">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24" style="color: var(--label-2)">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div v-else-if="!items.length" class="empty">
      <p>暂无翻译记录</p>
      <p class="hint">开启「入库自动翻译」或在上方批量翻译未翻译作品</p>
    </div>

    <!-- 记录列表（设计稿：单卡包裹 + .tr-item 分行） -->
    <div v-else class="card" style="padding: 6px">
      <div
        v-for="t in items"
        :key="t.id"
        class="tr-item"
        :class="{ clickable: t.status === 'success' && t.work_id }"
        :title="t.status === 'success' && t.work_id ? '点击查看作品' : undefined"
        @click="t.status === 'success' && openWork(t)"
      >
        <div class="tr-top">
          <span class="badge shrink-0" :class="statusBadge(t.status).cls">
            <Loader2 v-if="t.status === 'processing'" :size="10" class="animate-spin" />
            {{ statusBadge(t.status).label }}
          </span>
          <p class="tr-title">{{ t.title || t.rj_code || `#${t.work_id}` }}</p>
          <div class="tr-ops" @click.stop>
            <button
              v-if="t.status !== 'processing' && t.status !== 'pending'"
              class="opbtn"
              :title="t.status === 'success' ? '重新翻译' : '重试'"
              :disabled="busyId === t.id"
              @click="retry(t)"
            >
              <RotateCcw :size="12" />
            </button>
            <button
              class="opbtn x"
              title="删除记录"
              :disabled="busyId === t.id"
              @click="remove(t)"
            >
              <Trash2 :size="12" />
            </button>
          </div>
        </div>
        <p v-if="t.translated_title" class="tr-desc" style="color: var(--label-2)">
          <Languages :size="10" class="inline mr-1" style="color: var(--label-3)" />{{ t.translated_title }}
        </p>
        <p v-if="t.source_title && t.status !== 'success'" class="tr-desc">原文:{{ t.source_title.slice(0, 60) }}…</p>
        <p v-if="t.translated_desc" class="tr-desc">简介已翻译({{ t.translated_desc.length }} 字)</p>
        <p v-if="t.error" class="tr-err" :title="t.error">⚠️ {{ t.error }}</p>
        <p class="tr-foot">
          <template v-if="t.rj_code">{{ t.rj_code }}</template>
          <template v-if="t.updated_at"> · 更于 {{ t.updated_at.replace("T", " ").slice(0, 16) }}</template>
        </p>
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

<style scoped>
.page-wrap {
  max-width: 980px;
  margin: 0 auto;
  padding: 26px 28px 150px;
}
.page-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 22px;
}
.page-head h1 {
  font-size: 26px;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.page-head .sub {
  font-size: 13px;
  color: var(--label-2);
  margin-top: 3px;
}

/* 设计稿 .tr-item */
.tr-item {
  padding: 15px 18px;
}
.tr-item + .tr-item {
  margin-top: 0;
  border-top: 0.5px solid var(--sep);
}
.tr-item.clickable {
  cursor: pointer;
}
.tr-top {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
.tr-top .badge {
  flex-shrink: 0;
  margin-top: 2px;
}
.tr-title {
  flex: 1;
  min-width: 0;
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.5;
  color: var(--label);
}
.tr-item.clickable:hover .tr-title {
  color: var(--blue-deep);
}
.tr-ops {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}
.opbtn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: none;
  background: var(--fill-2);
  color: var(--label-2);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: background 0.2s;
}
.opbtn:hover:not(:disabled) {
  background: var(--fill-3);
  color: var(--label);
}
.opbtn.x:hover:not(:disabled) {
  background: rgba(255, 69, 58, 0.2);
  color: #ff6961;
}
.opbtn:disabled {
  opacity: 0.4;
  cursor: default;
}
.tr-desc {
  font-size: 12px;
  color: var(--label-3);
  margin-top: 6px;
  padding-left: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tr-err {
  font-size: 11.5px;
  color: #ff8a80;
  margin-top: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tr-foot {
  font-size: 11px;
  color: var(--label-3);
  margin-top: 8px;
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
}

@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}
</style>
