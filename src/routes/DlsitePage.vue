<script setup lang="ts">
import { ref, computed, onMounted, onActivated, nextTick } from "vue";
import { useRouter, useRoute } from "vue-router";
import { RefreshCw, Loader2 } from "lucide-vue-next";
import * as api from "../lib/api";
import type { DlsiteRankingItem } from "../lib/types";
import DlsiteWorkCard from "../components/dlsite/DlsiteWorkCard.vue";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();
const route = useRoute();

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
  saveScrollNow();
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

// KeepAlive 缓存本页：滚动时实时记录位置（离开时再读已被路由重置），返回时恢复
let savedScroll = 0;
let scrollEl: HTMLElement | null = null;
const onScroll = () => {
  // 跳转详情时内容替换会把 scrollTop 重置为 0，该重置事件不计入（仅列表路由时记录）
  if (route.name !== "dlsite") return;
  savedScroll = scrollEl?.scrollTop ?? 0;
};

/** 立即记录当前滚动位置（卡片点击跳转前调用，兜底 scroll 事件不触发的场景） */
function saveScrollNow() {
  savedScroll = scrollEl?.scrollTop ?? savedScroll;
}
onMounted(() => {
  load();
  scrollEl = document.querySelector("main");
  scrollEl?.addEventListener("scroll", onScroll, { passive: true });
});
onActivated(() => {
  // 详情页入库后返回：消费入库记录，原地补上「已入库」标识（KeepAlive 缓存不会重新拉数据）
  for (const item of items.value) {
    const id = worksStore.takeImported(item.rj_code);
    if (id != null) {
      item.work_id = id;
      item.download_status = "not_downloaded";
    }
  }
  nextTick(() => {
    const restore = () => {
      if (scrollEl) scrollEl.scrollTop = savedScroll;
    };
    restore();
    // 兜底：内容布局变化后再次校正（后台窗格 rAF 不触发，用 setTimeout）
    setTimeout(restore, 50);
    setTimeout(restore, 200);
  });
});
</script>

<template>
  <div class="page-wrap space-y-5">
    <!-- 页头：大标题 + 副标题 -->
    <div class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="page-h1">🏆 音声作品排行榜</h1>
        <p class="sub">
          来自 DLsite<template v-if="fetchedText"> · 更新于 {{ fetchedText }}</template>
        </p>
      </div>
      <button class="btn ghost sm" :disabled="refreshing" title="重新抓取全部榜单（约 10-20 秒）" @click="refresh">
        <Loader2 v-if="refreshing" :size="13" class="animate-spin" />
        <RefreshCw v-else :size="13" />
        刷新
      </button>
    </div>

    <!-- 工具栏：榜单周期 + 数量 -->
    <div class="flex flex-wrap items-center gap-2.5">
      <div class="seg sm">
        <button
          v-for="t in TERMS"
          :key="t.value"
          :class="{ on: term === t.value }"
          @click="switchTerm(t.value)"
        >
          {{ t.label }}
        </button>
      </div>
      <select
        v-model.number="limit"
        class="seg-select"
        title="显示条数"
        @change="load()"
      >
        <option v-for="n in LIMITS" :key="n" :value="n">Top {{ n }}</option>
      </select>
    </div>

    <!-- 错误 / 加载 / 空状态 -->
    <div v-if="error" class="card p-4 text-xs text-muted">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div v-else-if="!items.length" class="flex flex-col items-center justify-center py-24 text-muted text-sm gap-3">
      暂无榜单数据
      <button class="btn primary sm" :disabled="refreshing" @click="refresh">
        <Loader2 v-if="refreshing" :size="13" class="animate-spin" />
        <RefreshCw v-else :size="13" />
        立即获取
      </button>
    </div>

    <!-- 横版卡片网格 -->
    <div v-else class="rank-grid">
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

<style scoped>
.page-wrap {
  max-width: 1280px;
  margin: 0 auto;
  padding: 26px 28px 150px;
}
.page-h1 {
  font-size: 26px;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.sub {
  font-size: 13px;
  color: var(--label-2);
  margin-top: 3px;
}
@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}
.rank-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(330px, 1fr));
  gap: 18px;
}
</style>
