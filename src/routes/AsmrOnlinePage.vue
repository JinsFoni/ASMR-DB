<script setup lang="ts">
import { ref, computed, onMounted, onActivated, nextTick } from "vue";
import { useRouter, useRoute } from "vue-router";
import { Loader2, ChevronLeft, ChevronRight, Star, Heart, ListVideo, History, UserRound } from "lucide-vue-next";
import * as api from "../lib/api";
import type { AsmrOnlineWork, AsmrPlaylist, AsmrWorksPage } from "../lib/types";
import AsmrWorkCard from "../components/asmr/AsmrWorkCard.vue";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();
const addingId = ref<string | number | null>(null);
const route = useRoute();

const SOURCE_TABS = [
  { value: "popular", label: "热门" },
  { value: "recommend", label: "推荐" },
  { value: "all", label: "全部" },
];
const ORDERS = [
  { value: "release", label: "最新" },
  { value: "dl_count", label: "下载量" },
  { value: "rating", label: "评分" },
];
const ACCOUNT_TABS = [
  { value: "favorite", label: "收藏", icon: Heart },
  { value: "playlist", label: "播放列表", icon: ListVideo },
  { value: "history", label: "历史", icon: History },
];

const sourceTab = ref("popular");
const accountTab = ref<string | null>(null);
const order = ref("release");
const page = ref(1);

const items = ref<AsmrOnlineWork[]>([]);
const hasNext = ref(false);
const playlists = ref<AsmrPlaylist[] | null>(null);
const selectedPlaylist = ref<AsmrPlaylist | null>(null);
const loading = ref(false);
const error = ref("");

const pageTitle = computed(() => {
  if (!accountTab.value) return SOURCE_TABS.find((t) => t.value === sourceTab.value)?.label || "";
  const name = ACCOUNT_TABS.find((t) => t.value === accountTab.value)?.label || "";
  if (accountTab.value === "playlist" && selectedPlaylist.value) return `${name} · ${selectedPlaylist.value.name}`;
  return name;
});

async function load() {
  loading.value = true;
  error.value = "";
  try {
    if (accountTab.value) {
      const res = await api.asmrAccount(
        accountTab.value,
        page.value,
        accountTab.value === "playlist" ? selectedPlaylist.value?.id : undefined
      );
      applyWorks(res);
    } else {
      const res: AsmrWorksPage = await api.asmrBrowse(sourceTab.value, page.value, order.value);
      applyWorks(res);
    }
  } catch (e) {
    error.value = String(e);
    items.value = [];
    hasNext.value = false;
  } finally {
    loading.value = false;
  }
}

function applyWorks(res: AsmrWorksPage | { type: "playlists"; playlists: AsmrPlaylist[] }) {
  if (res.type === "playlists") {
    playlists.value = res.playlists;
    items.value = [];
    hasNext.value = false;
  } else {
    items.value = res.items;
    hasNext.value = res.hasNext;
    playlists.value = null;
  }
}

function switchSource(tab: string) {
  if (sourceTab.value === tab && !accountTab.value) return;
  sourceTab.value = tab;
  accountTab.value = null;
  selectedPlaylist.value = null;
  playlists.value = null;
  page.value = 1;
  if (tab === "all") order.value = "release";
  load();
}

function switchOrder(o: string) {
  if (order.value === o) return;
  order.value = o;
  page.value = 1;
  load();
}

function switchAccount(tab: string) {
  if (accountTab.value === tab) return;
  accountTab.value = tab;
  selectedPlaylist.value = null;
  page.value = 1;
  load();
}

function openPlaylist(p: AsmrPlaylist) {
  selectedPlaylist.value = p;
  page.value = 1;
  load();
}

function backToPlaylists() {
  selectedPlaylist.value = null;
  page.value = 1;
  load();
}

function open(item: AsmrOnlineWork) {
  saveScrollNow();
  if (item.local_work_id) router.push(`/work/${item.local_work_id}`);
  else router.push(`/asmr/work/${item.id}`);
}

/** 卡片入库：按 RJ 号抓取元数据入库，成功后原地变为已入库状态 */
async function addToLibrary(item: AsmrOnlineWork) {
  if (addingId.value || !item.rj_code) return;
  addingId.value = item.id;
  try {
    const res = await api.importByRj(item.rj_code);
    item.local_work_id = res.work.work.id;
    // 同步 Pinia store，否则切回作品库时因缓存看不到新入库的作品
    worksStore.fetchWorks();
    worksStore.loadTags();
  } catch (e) {
    alert(`入库失败: ${e}`);
  } finally {
    addingId.value = null;
  }
}

function prevPage() {
  if (page.value <= 1) return;
  page.value -= 1;
  load();
  window.scrollTo({ top: 0 });
}

function nextPage() {
  if (!hasNext.value) return;
  page.value += 1;
  load();
  window.scrollTo({ top: 0 });
}

// KeepAlive 缓存本页：滚动时实时记录位置（离开时再读已被路由重置），返回时恢复
let savedScroll = 0;
let scrollEl: HTMLElement | null = null;
const onScroll = () => {
  // 跳转详情时内容替换会把 scrollTop 重置为 0，该重置事件不计入（仅列表路由时记录）
  if (route.name !== "asmr") return;
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
  <div class="p-5 space-y-4">
    <!-- 工具栏：左侧内容切换 + 右侧账号切换 -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <div class="flex rounded-lg border border-bg-border overflow-hidden bg-bg-card">
          <button
            v-for="t in SOURCE_TABS"
            :key="t.value"
            class="px-3 py-1.5 text-xs transition-colors"
            :class="!accountTab && sourceTab === t.value ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
            @click="switchSource(t.value)"
          >
            {{ t.label }}
          </button>
        </div>
        <!-- 全部 tab 的排序切换 -->
        <div v-if="!accountTab && sourceTab === 'all'" class="flex rounded-lg border border-bg-border overflow-hidden bg-bg-card">
          <button
            v-for="o in ORDERS"
            :key="o.value"
            class="px-2.5 py-1.5 text-xs transition-colors"
            :class="order === o.value ? 'bg-bg-hover text-white' : 'text-muted hover:text-white'"
            @click="switchOrder(o.value)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-[10px] text-muted flex items-center gap-1">
          <UserRound :size="12" /> 账号
        </span>
        <div class="flex rounded-lg border border-bg-border overflow-hidden bg-bg-card">
          <button
            v-for="t in ACCOUNT_TABS"
            :key="t.value"
            class="px-3 py-1.5 text-xs transition-colors flex items-center gap-1"
            :class="accountTab === t.value ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
            @click="switchAccount(t.value)"
          >
            <component :is="t.icon" :size="12" /> {{ t.label }}
          </button>
        </div>
      </div>
    </div>

    <!-- 错误 / 加载 / 空状态 -->
    <div v-if="error" class="card p-4 text-xs text-muted">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div
      v-else-if="accountTab === 'playlist' && !selectedPlaylist"
      class="space-y-2"
    >
      <!-- 播放列表选择 -->
      <div v-if="!playlists?.length" class="text-center py-16 text-muted text-sm">
        暂无播放列表
      </div>
      <button
        v-for="p in playlists"
        :key="p.id"
        class="w-full card p-3 flex items-center gap-3 text-left hover:border-accent/50 transition-colors"
        @click="openPlaylist(p)"
      >
        <img
          v-if="p.main_cover_url"
          :src="p.main_cover_url"
          class="w-14 h-14 rounded-lg object-cover bg-bg-hover shrink-0"
        />
        <div v-else class="w-14 h-14 rounded-lg bg-bg-hover flex items-center justify-center text-muted/50 shrink-0">
          <ListVideo :size="20" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm text-white font-medium ellipsis-1">{{ p.name }}</div>
          <div class="text-[11px] text-muted mt-0.5">
            {{ p.works_count ?? 0 }} 部作品<template v-if="p.user_name"> · {{ p.user_name }}</template>
          </div>
          <div v-if="p.description" class="text-[11px] text-muted/70 mt-0.5 ellipsis-1">{{ p.description }}</div>
        </div>
        <ChevronRight :size="16" class="text-muted shrink-0" />
      </button>
    </div>
    <div v-else-if="!items.length" class="text-center py-16 text-muted text-sm">
      暂无数据
    </div>

    <!-- 播放列表详情的返回条 -->
    <div v-if="accountTab === 'playlist' && selectedPlaylist" class="flex items-center gap-2 text-xs">
      <button class="btn-ghost !px-2 !py-1" @click="backToPlaylists">
        <ChevronLeft :size="13" /> 返回列表
      </button>
      <span class="text-white/80">{{ pageTitle }}</span>
    </div>

    <!-- 卡片网格 -->
    <div v-if="!loading && items.length" class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(300px, 1fr))">
      <AsmrWorkCard
        v-for="item in items"
        :key="`${item.id}-${item.rj_code}`"
        :item="item"
        :adding="addingId === item.id"
        @open="open"
        @add="addToLibrary"
      />
    </div>

    <!-- 分页 -->
    <div v-if="!loading && !error && (items.length || page > 1)" class="flex items-center justify-center gap-3 pt-2">
      <button class="btn-outline !text-xs !py-1.5" :disabled="page <= 1" @click="prevPage">
        <ChevronLeft :size="13" /> 上一页
      </button>
      <span class="text-xs text-muted">第 {{ page }} 页</span>
      <button class="btn-outline !text-xs !py-1.5" :disabled="!hasNext" @click="nextPage">
        下一页 <ChevronRight :size="13" />
      </button>
    </div>

    <!-- 账号数据提示 -->
    <div v-if="!loading && accountTab && items.length" class="text-center text-[10px] text-muted/60 flex items-center justify-center gap-1">
      <Star :size="10" />
      {{ accountTab === "history" ? "本地播放历史（最近播放优先）" : "来自 asmr.one 账号数据" }}
    </div>
  </div>
</template>
