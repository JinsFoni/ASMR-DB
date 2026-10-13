<script setup lang="ts">
import { ref, computed, onMounted, onActivated, nextTick } from "vue";
import { useRouter, useRoute } from "vue-router";
import { Loader2, ChevronLeft, ChevronRight, Heart, ListVideo, History } from "lucide-vue-next";
import { imgUrl } from "../lib/api";
import * as api from "../lib/api";
import type { AsmrOnlineWork, AsmrPlaylist, AsmrWorksPage } from "../lib/types";
import AsmrWorkCard from "../components/asmr/AsmrWorkCard.vue";
import { useCardPlay } from "../composables/useCardPlay";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();
const addingId = ref<string | number | null>(null);
const route = useRoute();
const { playItem, isItemCurrent, isItemLoading } = useCardPlay();

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
        // 传响应式代理（items.value）而非原始数组：直接改原始对象不会触发卡片重渲染
        enrichMeta(items.value);
      }
}

let enrichSeq = 0;
/** 后台补齐卡片元数据（收藏数/评分/声优/发售日期/标签）：
 *  已入库作品直接取本地库（无外网请求），未入库的走 DLsite 统一预览接口（服务端短 TTL 缓存）。 */
async function enrichMeta(list: AsmrOnlineWork[]) {
  const seq = ++enrichSeq;
  // 本地库归一化成与 ScrapedWork 同形的补丁，两条路径共用一套合并逻辑
  const byRj = new Map(
    worksStore.works
      .filter((v) => v.work.rj_code)
      .map((v) => [
        v.work.rj_code.toUpperCase(),
        {
          rating: v.work.rating,
          wishlist_count: v.work.wishlist_count,
          sale_date: v.work.sale_date,
          actors: v.actors,
        },
      ])
  );
  for (const item of list) {
    if (!item.rj_code) continue;
    const rj = item.rj_code.toUpperCase();
    try {
      let meta = byRj.get(rj);
      if (!meta) {
        const s = await api.previewByRj(rj);
        if (seq !== enrichSeq) return;
        meta = {
          rating: s.rating,
          wishlist_count: s.wishlist_count,
          sale_date: s.sale_date,
          actors: s.actors,
        };
      }
      if (seq !== enrichSeq) return;
      if (meta.rating != null) item.rating = meta.rating;
      if (meta.wishlist_count != null) item.wishlist_count = meta.wishlist_count;
      if (meta.sale_date) item.release = meta.sale_date;
      if (meta.actors.length) item.vas = meta.actors;
      // 标签保留 asmr.one 原值（DLsite 的类型标签偏商城分类，卡片展示不友好）
    } catch {
      // 抓取失败（如 DLsite 下架）保持 asmr.one 原值，静默降级
    }
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
  // 详情页入库后返回：消费入库记录，原地补上「已入库」标识（KeepAlive 缓存不会重新拉数据）
  for (const item of items.value) {
    const id = item.rj_code ? worksStore.takeImported(item.rj_code) : null;
    if (id != null) item.local_work_id = id;
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
    <!-- 工具栏：左侧内容切换 + 右侧账号切换（设计稿 dual-tools，无大标题） -->
    <div class="dual-tools">
      <div class="seg">
        <button
          v-for="t in SOURCE_TABS"
          :key="t.value"
          :class="{ on: !accountTab && sourceTab === t.value }"
          @click="switchSource(t.value)"
        >
          {{ t.label }}
        </button>
      </div>
      <div class="flex items-center gap-3 flex-wrap">
        <span v-if="!accountTab && sourceTab === 'all'" class="seg" style="order: 2">
          <button
            v-for="o in ORDERS"
            :key="o.value"
            :class="{ on: order === o.value }"
            @click="switchOrder(o.value)"
          >
            {{ o.label }}
          </button>
        </span>
        <span class="acct-label">👤 账号</span>
        <div class="seg">
          <button
            v-for="t in ACCOUNT_TABS"
            :key="t.value"
            class="flex items-center gap-1.5"
            :class="{ on: accountTab === t.value }"
            @click="switchAccount(t.value)"
          >
            <component :is="t.icon" :size="13" /> {{ t.label }}
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
      class="pl-section"
    >
      <!-- 播放列表选择（设计稿：h2 标题 + pl-card 侧卡） -->
      <h2>
        <ListVideo :size="15" /> 播放列表
        <span v-if="playlists?.length" class="n">{{ playlists.length }}</span>
      </h2>
      <div v-if="!playlists?.length" class="text-center py-16 text-muted text-sm">
        暂无播放列表
      </div>
      <button
        v-for="p in playlists"
        :key="p.id"
        class="w-full card p-3.5 flex items-center gap-3.5 text-left hover:-translate-y-0.5 transition-transform"
        @click="openPlaylist(p)"
      >
        <img
          v-if="p.main_cover_url"
          :src="imgUrl(p.main_cover_url)"
          class="w-14 h-14 rounded-xl object-cover shrink-0"
          style="background: var(--fill-1)"
        />
        <div v-else class="w-14 h-14 rounded-xl flex items-center justify-center text-muted/50 shrink-0" style="background: var(--fill-1)">
          <ListVideo :size="20" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold ellipsis-1">{{ p.name }}</div>
          <div class="text-[11px] mt-0.5" style="color: var(--label-2)">
            {{ p.works_count ?? 0 }} 部作品<template v-if="p.user_name"> · {{ p.user_name }}</template>
          </div>
          <div v-if="p.description" class="text-[11px] mt-0.5 ellipsis-1" style="color: var(--label-3)">{{ p.description }}</div>
        </div>
        <ChevronRight :size="16" class="shrink-0" style="color: var(--label-3)" />
      </button>
    </div>
    <div v-else-if="!items.length" class="text-center py-16 text-muted text-sm">
      暂无数据
    </div>

    <!-- 播放列表详情的返回条 -->
    <div v-if="accountTab === 'playlist' && selectedPlaylist" class="flex items-center gap-2 text-xs">
      <button class="btn ghost sm" @click="backToPlaylists">
        <ChevronLeft :size="13" /> 返回列表
      </button>
      <span style="color: var(--label)">{{ pageTitle }}</span>
    </div>

    <!-- 卡片网格（设计稿 .grid） -->
    <div v-if="!loading && items.length" class="grid-works">
      <AsmrWorkCard
        v-for="item in items"
        :key="`${item.id}-${item.rj_code}`"
        :item="item"
        :adding="addingId === item.id"
        :playable="!!item.duration_sec || !!item.local_work_id"
        :play-loading="isItemLoading(item)"
        :play-current="isItemCurrent(item)"
        @open="open"
        @add="addToLibrary"
        @play="playItem"
      />
    </div>

    <!-- 分页（设计稿 .pager） -->
    <div v-if="!loading && !error && (items.length || page > 1)" class="pager">
      <button :disabled="page <= 1" @click="prevPage">‹ 上一页</button>
      <button class="on">{{ page }}</button>
      <button :disabled="!hasNext" @click="nextPage">下一页 ›</button>
    </div>

    <!-- 底部数据来源提示（设计稿 .foot-hint） -->
    <div v-if="!loading && !error && items.length" class="foot-hint">
      来自 asmr.one {{ accountTab ? "账号数据" : "在线数据" }} · api.asmr-200.com
    </div>
  </div>
</template>

<style scoped>
.page-wrap {
  max-width: 1280px;
  margin: 0 auto;
  padding: 30px 28px 150px;
}
@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}
.dual-tools {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
}
.acct-label {
  font-size: 12px;
  color: var(--label-3);
}
.grid-works {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
  gap: 22px;
}
.pager {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 14px;
  margin-top: 30px;
}
.pager button {
  height: 34px;
  padding: 0 16px;
  border: none;
  border-radius: 999px;
  background: var(--fill-1);
  color: var(--label-2);
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}
.pager button:hover:not(:disabled) {
  background: var(--fill-2);
  color: var(--label);
}
.pager button.on {
  background: var(--blue);
  color: #fff;
}
.pager button:disabled {
  opacity: 0.4;
  cursor: default;
}
/* 播放列表分区（设计稿 .pl-section） */
.pl-section h2 {
  font-size: 17px;
  font-weight: 700;
  margin-bottom: 14px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--label);
}
.pl-section h2 .n {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--label-3);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 2px 10px;
}
/* 底部来源提示（设计稿 .foot-hint） */
.foot-hint {
  text-align: center;
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 22px;
}
</style>
