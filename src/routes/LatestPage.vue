<script setup lang="ts">
import { ref, computed, onMounted, onActivated, nextTick } from "vue";
import { useRouter, useRoute } from "vue-router";
import { Loader2 } from "lucide-vue-next";
import * as api from "../lib/api";
import type { AsmrOnlineWork, AsmrWorksPage } from "../lib/types";
import AsmrWorkCard from "../components/asmr/AsmrWorkCard.vue";
import { useCardPlay } from "../composables/useCardPlay";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();
const route = useRoute();
const { playItem, isItemCurrent, isItemLoading } = useCardPlay();

const ORDERS = [
  { value: "release", label: "最新", hint: "最新发售" },
  { value: "trend", label: "人气", hint: "人气最高" },
  { value: "dl_d", label: "销量", hint: "销量最高" },
  { value: "rate_d", label: "评分", hint: "评分最高" },
  { value: "review_d", label: "赏析", hint: "赏析最多" },
];

const order = ref("release");
const orderHint = computed(() => ORDERS.find((o) => o.value === order.value)?.hint ?? "");
const items = ref<AsmrOnlineWork[]>([]);
const page = ref(1);
const hasNext = ref(false);
const loading = ref(false);
const error = ref("");
const addingId = ref<string | number | null>(null);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const res: AsmrWorksPage = await api.dlsiteLatest(page.value, order.value);
    items.value = res.items;
    hasNext.value = res.hasNext;
    // 传响应式代理（items.value）而非原始数组：直接改原始对象不会触发卡片重渲染
    const seq = ++enrichSeq;
    enrichMeta(items.value, seq);
    probeAudio(items.value, seq);
  } catch (e) {
    error.value = String(e);
    items.value = [];
    hasNext.value = false;
  } finally {
    loading.value = false;
  }
}

let enrichSeq = 0;
/** asmr.one 收录探测结果（RJ → 有无可播音频），DLsite 列表数据本身不携带该信息 */
const audioProbed = ref(new Map<string, boolean>());

/** 卡片有无可播音频：本地有音轨，或 asmr.one 已收录（探测写回） */
function hasAudio(item: AsmrOnlineWork): boolean {
  if (!item.rj_code) return false;
  const rj = item.rj_code.toUpperCase();
  const local = worksStore.works.find((v) => v.work.rj_code?.toUpperCase() === rj);
  if (local && local.track_count > 0) return true;
  return audioProbed.value.get(rj) === true;
}

/** 后台探测列表里每个未入库作品在 asmr.one 的收录情况（HEAD 探测很轻，全量并发）。
 *  每个结果单独触发一次 30 张卡的重渲染会形成渲染风暴（实测滚动掉帧的元凶），
 *  改为收集完统一批量写回，只重渲染一次。 */
async function probeAudio(list: AsmrOnlineWork[], seq: number) {
  const results = await Promise.all(
    list.map(async (item) => {
      if (!item.rj_code) return null;
      const rj = item.rj_code.toUpperCase();
      if (audioProbed.value.has(rj)) return null;
      return [rj, await api.hasAsmroneAudio(rj)] as const;
    })
  );
  if (seq !== enrichSeq) return;
  let changed = false;
  const next = new Map(audioProbed.value);
  for (const r of results) {
    if (r) {
      next.set(r[0], r[1]);
      changed = true;
    }
  }
  if (changed) audioProbed.value = next;
}

/** 后台补齐卡片元数据（收藏数/评分/声优/发售日期，与在线页同一策略）：
 *  已入库作品直接取本地库（无外网请求），未入库的走 DLsite 统一预览接口（服务端短 TTL 缓存）。 */
async function enrichMeta(list: AsmrOnlineWork[], seq: number) {
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
          tags: v.tags.map((t) => t.name),
          duration_min: v.total_duration_sec ? Math.round(v.total_duration_sec / 60) : null,
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
          tags: s.tags,
          duration_min: s.duration_min,
        };
      }
      if (seq !== enrichSeq) return;
      if (meta.rating != null) item.rating = meta.rating;
      if (meta.wishlist_count != null) item.wishlist_count = meta.wishlist_count;
      if (meta.sale_date) item.release = meta.sale_date;
      if (meta.actors.length) item.vas = meta.actors;
      if (meta.tags.length) item.tags = meta.tags;
      if (meta.duration_min) item.duration_sec = meta.duration_min * 60;
    } catch {
      // 抓取失败（如 DLsite 下架）保持列表页原值，静默降级
    }
  }
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

function open(item: AsmrOnlineWork) {
  if (item.local_work_id) router.push(`/work/${item.local_work_id}`);
  else if (item.rj_code) router.push(`/dlsite/work/${item.rj_code}`);
  else router.push(`/asmr/work/${item.id}`);
}

function switchOrder(o: string) {
  if (order.value === o) return;
  order.value = o;
  page.value = 1;
  load();
  window.scrollTo({ top: 0 });
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

// KeepAlive 缓存本页：滚动时实时记录位置，返回时恢复
let savedScroll = 0;
let scrollEl: HTMLElement | null = null;
const onScroll = () => {
  if (route.name !== "latest") return;
  savedScroll = scrollEl?.scrollTop ?? 0;
};
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
    <!-- 切换栏 + 标题（跟随切换：最新发售/人气最高/销量最高/评分最高/赏析最多） -->
    <div class="dual-tools">
      <div class="seg">
        <button
          v-for="o in ORDERS"
          :key="o.value"
          :class="{ on: order === o.value }"
          @click="switchOrder(o.value)"
        >
          {{ o.label }}
        </button>
      </div>
      <h1 class="page-title">{{ orderHint }}</h1>
    </div>

    <!-- 错误 / 加载 / 空状态 -->
    <div v-if="error" class="card p-4 text-xs text-muted">⚠️ {{ error }}</div>
    <div v-else-if="loading" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="24" class="animate-spin" />
    </div>
    <div v-else-if="!items.length" class="text-center py-16 text-muted text-sm">
      暂无数据
    </div>

    <!-- 卡片网格（与在线页同一卡片与网格） -->
    <div v-if="!loading && items.length" class="grid-works">
      <!-- 播放按钮显隐 = 真实有无音频：本地已入库看 track_count，
           未入库的由 HEAD /api/asmr/tracks 探测 asmr.one 收录情况（enrichMeta 内逐个写回） -->
      <AsmrWorkCard
        v-for="item in items"
        :key="`${item.id}-${item.rj_code}`"
        :item="item"
        :adding="addingId === item.id"
        :playable="hasAudio(item)"
        :play-loading="isItemLoading(item)"
        :play-current="isItemCurrent(item)"
        @open="open"
        @add="addToLibrary"
        @play="playItem"
      />
    </div>

    <!-- 分页 -->
    <div v-if="!loading && !error && (items.length || page > 1)" class="pager">
      <button :disabled="page <= 1" @click="prevPage">‹ 上一页</button>
      <button class="on">{{ page }}</button>
      <button :disabled="!hasNext" @click="nextPage">下一页 ›</button>
    </div>

    <!-- 底部数据来源提示 -->
    <div v-if="!loading && !error && items.length" class="foot-hint">
      来自 DLsite · 评分与收藏数后台逐卡补齐
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
.page-title {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.01em;
  color: var(--label);
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
.foot-hint {
  text-align: center;
  font-size: 11px;
  color: var(--label-3);
  padding-top: 6px;
}
</style>
