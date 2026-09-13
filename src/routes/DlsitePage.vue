<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { RefreshCw, Loader2, Trophy } from "lucide-vue-next";
import * as api from "../lib/api";
import type { DlsiteRankingItem } from "../lib/types";
import DlsiteWorkCard from "../components/dlsite/DlsiteWorkCard.vue";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();

const TERMS = [
  { value: "day", label: "日榜" },
  { value: "week", label: "周榜" },
  { value: "month", label: "月榜" },
  { value: "year", label: "年度" },
  { value: "total", label: "累计" },
];
const LIMITS = [20, 50, 100];

const term = ref("day");
const limit = ref(20);
const items = ref<DlsiteRankingItem[]>([]);
const lastFetched = ref<string | null>(null);
const loading = ref(false);
const refreshing = ref(false);
const addingRj = ref<string | null>(null);
const error = ref("");

/** SQLite UTC 时间转本地时间显示 */
const fetchedText = computed(() => {
  if (!lastFetched.value) return "";
  const d = new Date(lastFetched.value.replace(" ", "T") + "Z");
  return isNaN(d.getTime()) ? lastFetched.value : d.toLocaleString();
});

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const res = await api.getDlsiteRanking(term.value, limit.value);
    items.value = res.items;
    lastFetched.value = res.lastFetched;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

/** 手动触发全量抓取（5 个周期，约 10-20 秒） */
async function refresh() {
  refreshing.value = true;
  error.value = "";
  try {
    await api.refreshDlsiteRanking();
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    refreshing.value = false;
  }
}

function switchTerm(value: string) {
  if (term.value === value) return;
  term.value = value;
  load();
}

/** 已入库 → 本地详情；未入库 → 在线预览详情 */
function open(item: DlsiteRankingItem) {
  if (item.work_id) router.push(`/work/${item.work_id}`);
  else router.push(`/dlsite/work/${item.rj_code}`);
}

/** 卡片入库：成功后原地更新为已入库状态，并同步作品库列表 */
async function addToLibrary(item: DlsiteRankingItem) {
  if (addingRj.value) return;
  addingRj.value = item.rj_code;
  try {
    const res = await api.importByRj(item.rj_code);
    item.work_id = res.work.work.id;
    item.download_status = "not_downloaded";
    // 同步 Pinia store，否则切回作品库时因缓存不会重新拉取，看不到新入库的作品
    worksStore.fetchWorks();
    worksStore.loadTags();
  } catch (e) {
    alert(`入库失败: ${e}`);
  } finally {
    addingRj.value = null;
  }
}

onMounted(load);
</script>

<template>
  <div class="p-5 space-y-4">
    <!-- 工具栏：左侧说明 + 右侧榜单切换/数量/刷新 -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="text-xs text-muted flex items-center gap-1.5">
        <Trophy :size="13" class="text-accent-light" />
        音声作品排行榜
        <span v-if="fetchedText"> · 更新于 {{ fetchedText }}</span>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <div class="flex rounded-lg border border-bg-border overflow-hidden bg-bg-card">
          <button
            v-for="t in TERMS"
            :key="t.value"
            class="px-3 py-1.5 text-xs transition-colors"
            :class="term === t.value ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
            @click="switchTerm(t.value)"
          >
            {{ t.label }}
          </button>
        </div>
        <select
          v-model.number="limit"
          class="input !text-xs !py-1.5 !w-auto"
          title="显示条数"
          @change="load()"
        >
          <option v-for="n in LIMITS" :key="n" :value="n">Top {{ n }}</option>
        </select>
        <button class="btn-outline !text-xs !py-1.5" :disabled="refreshing" title="重新抓取全部榜单（约 10-20 秒）" @click="refresh">
          <Loader2 v-if="refreshing" :size="13" class="animate-spin" />
          <RefreshCw v-else :size="13" />
          刷新
        </button>
      </div>
    </div>

    <!-- 错误 / 加载 / 空状态 -->
    <div v-if="error" class="card p-4 text-xs text-muted">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div v-else-if="!items.length" class="flex flex-col items-center justify-center py-24 text-muted text-sm gap-3">
      暂无榜单数据
      <button class="btn-primary !text-xs" :disabled="refreshing" @click="refresh">
        <Loader2 v-if="refreshing" :size="13" class="animate-spin" />
        <RefreshCw v-else :size="13" />
        立即获取
      </button>
    </div>

    <!-- 横版卡片网格 -->
    <div v-else class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(300px, 1fr))">
      <DlsiteWorkCard
        v-for="item in items"
        :key="`${term}-${item.rj_code}`"
        :item="item"
        :adding="addingRj === item.rj_code"
        @open="open"
        @add="addToLibrary"
      />
    </div>
  </div>
</template>
