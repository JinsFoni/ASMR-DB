<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ArrowLeft,
  Globe,
  Monitor,
  Copy,
  Check,
  Loader2,
  ImageOff,
  Headphones,
  BookmarkCheck,
  RotateCw,
  Library,
} from "lucide-vue-next";
import * as api from "../lib/api";
import type { AsmrOnlineWork, AsmrTreeNode, Track, WorkView } from "../lib/types";
import TrackList from "../components/detail/TrackList.vue";
import { usePlayerStore } from "../stores/player";

const route = useRoute();
const router = useRouter();
const player = usePlayerStore();
/** 路由参数变化时组件会复用（如 详情→详情 跳转），用 computed 响应新 id */
const workId = computed(() => String(route.params.id || ""));

const detail = ref<AsmrOnlineWork | null>(null);
const tracks = ref<Track[]>([]);
const fileTree = ref<AsmrTreeNode[] | null>(null);
const tracksLoading = ref(false);
const loading = ref(true);
const error = ref("");
const copied = ref(false);
const onlineLoading = ref(false);
const importing = ref(false);

const title = computed(() => detail.value?.title || detail.value?.rj_code || `#${workId}`);

async function load() {
  loading.value = true;
  error.value = "";
  copied.value = false;
  tracks.value = [];
  fileTree.value = null;
  try {
    detail.value = await api.asmrWorkDetail(workId.value);
    loadTracks();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

/** 详情 → 详情 跳转时路由参数变化但组件复用，必须重新加载 */
watch(workId, (v, old) => {
  if (v && v !== old) load();
});

/** 拉取 asmr.one 音轨列表与完整文件树（音频列表/全部文件/在线试听共用） */
async function loadTracks() {
  if (!detail.value || tracksLoading.value) return;
  tracksLoading.value = true;
  try {
    const key = detail.value.rj_code || String(detail.value.id);
    const [t, tree] = await Promise.all([
      api.listAsmroneTracks(key),
      api.listAsmroneTree(key).catch(() => null),
    ]);
    tracks.value = t;
    fileTree.value = tree;
  } catch {
    tracks.value = [];
    fileTree.value = null;
  } finally {
    tracksLoading.value = false;
  }
}

onMounted(load);

async function copyRj() {
  if (!detail.value?.rj_code) return;
  await navigator.clipboard.writeText(detail.value.rj_code);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}

/** 构造临时 WorkView 供播放器/音轨列表展示（在线音轨不写播放进度） */
const fakeView = computed<WorkView>(() => {
  const w = detail.value;
  return {
    work: {
      id: 0,
      rj_code: w?.rj_code || `ASMR-${w?.id ?? workId}`,
      title_ja: w?.title || null,
      title_zh: null,
      title_en: null,
      circle_name: w?.circle_name || null,
      circle_id: null,
      cover_url: w?.cover_url || null,
      work_type: "voice",
      price: w?.price ?? null,
      sale_date: w?.release ?? null,
      description: w?.description ?? null,
      description_zh: w?.description ?? null,
      age_class: null,
      duration_min: w?.duration_sec ? Math.round(w.duration_sec / 60) : null,
      file_size_mb: null,
      dlsite_url: w?.rj_code ? `https://www.dlsite.com/maniax/work/=/product_id/${w.rj_code}.html` : null,
      created_at: null,
      updated_at: null,
    },
    tags: [],
    actors: w?.vas || [],
    download_status: "not_downloaded",
    local_path: null,
    track_count: tracks.value.length,
    total_duration_sec: w?.duration_sec || 0,
    group: null,
  };
});

/** 在线试听：拉取 asmr.one 音轨并进入播放器 */
async function playOnline() {
  if (!detail.value || onlineLoading.value) return;
  onlineLoading.value = true;
  try {
    if (!tracks.value.length) await loadTracks();
    if (!tracks.value.length) {
      alert("asmr.one 未找到音频文件");
      return;
    }
    player.playFromWork(fakeView.value, tracks.value, 0, 0);
    router.push({ name: "player" });
  } catch (e) {
    alert(`在线试听失败: ${e}`);
  } finally {
    onlineLoading.value = false;
  }
}

/** 入库：按 RJ 号抓取元数据保存后跳转本地作品详情 */
async function importWork() {
  if (!detail.value || importing.value || !detail.value.rj_code) return;
  importing.value = true;
  try {
    const res = await api.importByRj(detail.value.rj_code);
    router.push(`/work/${res.work.work.id}`);
  } catch (e) {
    alert(`入库失败: ${e}`);
    importing.value = false;
  }
}

function openDlsite() {
  if (!detail.value?.rj_code) return;
  window.open(
    `https://www.dlsite.com/maniax/work/=/product_id/${detail.value.rj_code}.html`,
    "_blank",
    "noopener"
  );
}

function openAsmrOneWeb() {
  window.open(`https://www.asmr.one/work/${workId}`, "_blank", "noopener");
}

function goLocal() {
  if (detail.value?.local_work_id) router.push(`/work/${detail.value.local_work_id}`);
}
</script>

<template>
  <div v-if="loading" class="flex items-center justify-center py-24 text-muted">
    <Loader2 :size="24" class="animate-spin" />
  </div>

  <div v-else-if="error" class="p-5 max-w-5xl mx-auto space-y-4">
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="router.back()">
      <ArrowLeft :size="14" /> 返回
    </button>
    <div class="card p-6 text-sm text-muted space-y-3">
      <div>⚠️ 获取在线作品数据失败</div>
      <div class="text-xs">{{ error }}</div>
      <div class="flex gap-2">
        <button class="btn-outline !text-xs" :disabled="loading" @click="load">
          <RotateCw :size="13" /> 重试
        </button>
        <button class="btn-ghost !text-xs" @click="openAsmrOneWeb">
          <Monitor :size="13" /> 打开 ASMR ONE 网页
        </button>
      </div>
    </div>
  </div>

  <div v-else-if="detail" class="p-5 space-y-5 max-w-5xl mx-auto">
    <!-- 返回 -->
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="router.back()">
      <ArrowLeft :size="14" /> 返回
    </button>

    <div class="flex gap-5">
      <!-- 封面（完整不裁切） -->
      <div class="w-72 shrink-0 self-center rounded-xl overflow-hidden bg-bg-hover border border-bg-border">
        <img v-if="detail.cover_url" :src="detail.cover_url" class="w-full h-auto" />
        <div v-else class="w-72 aspect-[4/3] flex items-center justify-center text-muted/40">
          <ImageOff :size="40" />
        </div>
      </div>

      <!-- 信息 -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 text-xs">
          <span v-if="detail.rj_code" class="font-mono text-muted">{{ detail.rj_code }}</span>
          <button v-if="detail.rj_code" class="text-muted hover:text-white" title="复制RJ号" @click="copyRj">
            <Check v-if="copied" :size="13" class="text-green-400" />
            <Copy v-else :size="13" />
          </button>
          <span
            v-if="detail.local_work_id"
            class="px-1.5 py-0.5 rounded text-[10px] border border-yellow-500/50 text-yellow-400 bg-yellow-500/10 flex items-center gap-1 cursor-pointer"
            title="该作品已在作品库中，点击查看本地详情"
            @click="goLocal"
          >
            <BookmarkCheck :size="10" /> 已入库
          </span>
        </div>

        <h1 class="text-xl font-bold text-white mt-2 leading-snug">{{ title }}</h1>

        <!-- 元数据行 -->
        <div class="flex flex-wrap gap-x-5 gap-y-1.5 mt-4 text-xs text-muted">
          <span v-if="detail.circle_name">🏢 社团：{{ detail.circle_name }}</span>
          <span v-if="detail.release">📅 发售：{{ detail.release.slice(0, 10) }}</span>
          <span v-if="detail.rating != null && detail.rating > 0">
            ⭐ {{ detail.rating.toFixed(1) }}<template v-if="detail.rate_count">（{{ detail.rate_count }} 人评价）</template>
          </span>
          <span v-if="detail.duration_sec">⏱ {{ Math.round(detail.duration_sec / 60) }}分</span>
          <span v-if="detail.price">💰 {{ detail.price.toLocaleString() }}円</span>
        </div>

        <div v-if="detail.vas.length" class="mt-2 text-xs text-muted">
          🎤 声优：<span class="text-white/80">{{ detail.vas.join(" / ") }}</span>
        </div>

        <!-- 标签（在线预览只读） -->
        <div v-if="detail.tags.length" class="flex flex-wrap gap-1.5 mt-3">
          <span
            v-for="t in detail.tags"
            :key="t"
            class="px-2 py-0.5 rounded text-[11px] bg-bg-hover text-muted border border-bg-border"
          >
            {{ t }}
          </span>
        </div>

        <!-- 操作按钮 -->
        <div class="flex flex-wrap gap-2 mt-5">
          <button class="btn-primary" :disabled="onlineLoading" @click="playOnline">
            <Loader2 v-if="onlineLoading" :size="14" class="animate-spin" />
            <Headphones v-else :size="14" />
            在线试听
          </button>
          <button
            v-if="!detail.local_work_id && detail.rj_code"
            class="btn-outline"
            :disabled="importing"
            title="将此作品加入作品库"
            @click="importWork"
          >
            <Loader2 v-if="importing" :size="14" class="animate-spin" />
            <BookmarkCheck v-else :size="14" />
            入库
          </button>
          <button v-if="detail.local_work_id" class="btn-outline" @click="goLocal">
            <Library :size="14" /> 本地详情
          </button>
          <button v-if="detail.rj_code" class="btn-ghost" @click="openDlsite">
            <Globe :size="14" /> DLsite 页面
          </button>
          <button class="btn-ghost" @click="openAsmrOneWeb">
            <Monitor :size="14" /> ASMR One
          </button>
        </div>
      </div>
    </div>

    <!-- 简介 -->
    <div v-if="detail.description" class="card p-4">
      <div class="text-xs font-semibold text-white mb-2">📝 简介</div>
      <p class="text-xs text-muted leading-relaxed whitespace-pre-line max-h-48 overflow-y-auto">
        {{ detail.description }}
      </p>
    </div>

    <!-- 音轨（文件）列表 -->
    <div v-if="tracksLoading" class="card p-6 flex items-center justify-center text-muted">
      <Loader2 :size="18" class="animate-spin" />
      <span class="text-xs ml-2">正在获取音轨列表…</span>
    </div>
    <TrackList v-else :tracks="tracks" :work="fakeView" online :tree="fileTree" />
  </div>
</template>
