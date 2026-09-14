<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ArrowLeft,
  Download,
  Play,
  FolderOpen,
  FolderPlus,
  Globe,
  Copy,
  Check,
  Loader2,
  ImageOff,
  Trash2,
  Music,
  Monitor,
  Headphones,
  Languages,
} from "lucide-vue-next";
import * as api from "../lib/api";
import type { WorkView, Track, AsmrTreeNode, AsmrDownloadFile } from "../lib/types";
import TrackList from "../components/detail/TrackList.vue";
import TagEditor from "../components/detail/TagEditor.vue";
import AsmrFileTree from "../components/AsmrFileTree.vue";
import FolderPickerModal from "../components/common/FolderPickerModal.vue";
import { usePlayerStore } from "../stores/player";
import { useWorksStore } from "../stores/works";
import { useDownloadStore } from "../stores/download";

const route = useRoute();
const router = useRouter();
const player = usePlayerStore();
const worksStore = useWorksStore();
const downloadStore = useDownloadStore();

const workId = Number(route.params.id);
const view = ref<WorkView | null>(null);
const tracks = ref<Track[]>([]);
const loading = ref(true);
const copied = ref(false);
const scanningTracks = ref(false);
const onlineLoading = ref(false);
const translating = ref(false);

// 下载链接输入
const showUrlInput = ref(false);
const dlUrl = ref("");
const downloading = ref(false);

const title = computed(
  () => view.value?.work.title_zh || view.value?.work.title_ja || view.value?.work.rj_code
);

async function load() {
  loading.value = true;
  try {
    view.value = await api.getWork(workId);
    if (view.value) {
      tracks.value = await api.listTracks(workId);
    }
  } finally {
    loading.value = false;
  }
}

onMounted(load);

async function copyRj() {
  if (!view.value) return;
  await navigator.clipboard.writeText(view.value.work.rj_code);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}

async function startDownload() {
  if (!view.value) return;
  if (!dlUrl.value.trim()) {
    showUrlInput.value = true;
    return;
  }
  downloading.value = true;
  try {
    await downloadStore.startDownload(view.value.work.id, dlUrl.value.trim());
    showUrlInput.value = false;
  } catch (e) {
    alert(`下载失败: ${e}`);
  } finally {
    downloading.value = false;
  }
}

async function deleteWork() {
  if (!view.value) return;
  if (!confirm(`确定删除「${title.value}」？将从数据库移除该作品。`)) return;
  await api.deleteWork(view.value.work.id);
  worksStore.removeWork(view.value.work.id);
  router.push({ name: "home" });
}

async function playAll() {
  if (!view.value || !tracks.value.length) return;
  const progress = await api.getPlayProgress(view.value.work.id);
  let startIndex = 0;
  let initialTime = 0;
  if (progress) {
    const idx = tracks.value.findIndex((t) => t.id === progress.track_id);
    if (idx >= 0) {
      startIndex = idx;
      initialTime = progress.last_position || 0;
    }
  }
  player.playFromWork(view.value, tracks.value, startIndex, initialTime);
  router.push({ name: "player" });
}

async function openLocal() {
  if (view.value?.local_path) await api.openFolder(view.value.local_path);
  else alert("该作品还没有本地文件。");
}

async function scanTracks() {
  if (!view.value) return;
  scanningTracks.value = true;
  try {
    const res = await api.scanAudioTracks(view.value.work.id);
    if (!res.ok) alert(res.message || "扫描失败");
    else await load();
  } catch (e) {
    alert(`扫描音轨失败: ${e}`);
  } finally {
    scanningTracks.value = false;
  }
}

// 目录选择器（服务端目录浏览）
const showFolderPicker = ref(false);

async function bindLocal() {
  if (!view.value) return;
  showFolderPicker.value = true;
}

async function handleFolderPicked(dir: string) {
  if (!view.value) return;
  try {
    const res = await api.bindLocalFolder(view.value.work.rj_code, dir);
    alert(`关联成功！扫描到 ${res.trackCount} 首音轨${res.groupName ? `，已自动分配分组「${res.groupName}」` : ""}`);
    await load();
    worksStore.fetchWorks();
    worksStore.loadGroups();
  } catch (e) {
    alert(`关联失败: ${e}`);
  }
}

/** LLM 翻译：把作品的日文标题/简介加入翻译队列 */
async function translateWork() {
  if (!view.value || translating.value) return;
  translating.value = true;
  try {
    const res = await api.enqueueTranslation(view.value.work.id);
    alert(res.message || "已加入翻译队列");
  } catch (e) {
    alert(`加入翻译队列失败: ${e}`);
  } finally {
    translating.value = false;
  }
}

async function openDlsite() {
  const w = view.value?.work;
  const url = w?.dlsite_url || (w?.rj_code ? `https://www.dlsite.com/maniax/work/=/product_id/${w.rj_code}.html` : null);
  if (url) window.open(url, "_blank", "noopener");
}

async function openAsmrOne() {
  const rj = view.value?.work.rj_code;
  if (rj) window.open(`https://www.asmr.one/work/${rj}`, "_blank", "noopener");
}

async function playOnline() {
  if (!view.value) return;
  onlineLoading.value = true;
  try {
    const s = await api.getSettings();
    if (!s.asmr_one_token) {
      alert("请先在「设置 -> asmr.one」登录获取 Token 后再试");
      return;
    }
    const onlineTracks = await api.listAsmroneTracks(view.value.work.rj_code);
    if (!onlineTracks.length) {
      alert("asmr.one 未找到音频文件");
      return;
    }
    player.playFromWork(view.value, onlineTracks, 0, 0);
    router.push({ name: "player" });
  } catch (e) {
    alert(`在线试听失败: ${e}`);
  } finally {
    onlineLoading.value = false;
  }
}

// asmr.one 文件树下载
const showFileTree = ref(false);
const treeNodes = ref<AsmrTreeNode[]>([]);
const treeLoading = ref(false);

async function openAsmrDownload() {
  if (!view.value) return;
  const s = await api.getSettings();
  if (!s.asmr_one_token) {
    alert("请先在「设置 -> asmr.one」登录获取 Token 后再试");
    return;
  }
  showFileTree.value = true;
  treeLoading.value = true;
  try {
    treeNodes.value = await api.listAsmroneTree(view.value.work.rj_code);
  } catch (e) {
    alert(`获取 asmr.one 文件树失败: ${e}`);
    showFileTree.value = false;
  } finally {
    treeLoading.value = false;
  }
}

async function handleAsmrDownload(files: AsmrDownloadFile[]) {
  if (!view.value) return;
  showFileTree.value = false;
  try {
    await api.downloadAsmroneFiles(view.value.work.id, files);
    alert(`已开始从 asmr.one 下载 ${files.length} 个文件，可在「下载管理」查看实时进度。`);
    // 更新本地视图状态
    view.value.download_status = "downloading";
  } catch (e) {
    alert(`下载失败: ${e}`);
  }
}

function onTagsUpdated() {
  load();
}
</script>

<template>
  <div v-if="loading" class="flex items-center justify-center py-24 text-muted">
    <Loader2 :size="24" class="animate-spin" />
  </div>

  <div v-else-if="!view" class="p-5 text-center text-muted">作品不存在或已被删除</div>

  <div v-else class="p-5 space-y-5 max-w-5xl mx-auto">
    <!-- 返回 -->
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="router.back()">
      <ArrowLeft :size="14" /> 返回
    </button>

    <div class="flex gap-5">
      <!-- 封面（完整封面不裁切，与横版卡片一致；加宽并垂直居中避免信息列下方大片空白） -->
      <div class="w-72 shrink-0 self-center rounded-xl overflow-hidden bg-bg-hover border border-bg-border">
        <img v-if="view.work.cover_url" :src="view.work.cover_url" class="w-full h-auto" />
        <div v-else class="w-72 aspect-[4/3] flex items-center justify-center text-muted/40">
          <ImageOff :size="40" />
        </div>
      </div>

      <!-- 信息 -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 text-xs">
          <span class="font-mono text-muted">{{ view.work.rj_code }}</span>
          <button class="text-muted hover:text-white" title="复制RJ号" @click="copyRj">
            <Check v-if="copied" :size="13" class="text-green-400" />
            <Copy v-else :size="13" />
          </button>
          <span
            class="px-1.5 py-0.5 rounded text-[10px] border"
            :class="
              view.download_status === 'downloaded'
                ? 'border-green-500/50 text-green-400 bg-green-500/10'
                : view.download_status === 'downloading'
                ? 'border-accent/50 text-accent-light bg-accent/10'
                : 'border-bg-border text-muted bg-bg-hover'
            "
          >
            {{ view.download_status === "downloaded" ? "已下载" : view.download_status === "downloading" ? "下载中" : "未下载" }}
          </span>
        </div>

        <h1 class="text-xl font-bold text-white mt-2 leading-snug">{{ title }}</h1>
        <div v-if="view.work.title_ja && view.work.title_ja !== title" class="text-sm text-muted mt-0.5">
          {{ view.work.title_ja }}
        </div>

        <!-- 元数据行 -->
        <div class="flex flex-wrap gap-x-5 gap-y-1.5 mt-4 text-xs text-muted">
          <span v-if="view.work.circle_name">🏢 社团：{{ view.work.circle_name }}</span>
          <span v-if="view.work.sale_date">📅 发售：{{ view.work.sale_date }}</span>
          <span v-if="view.work.price">💰 {{ view.work.price.toLocaleString() }}円</span>
          <span v-if="view.work.duration_min">⏱ {{ view.work.duration_min }}分</span>
          <span v-if="view.work.file_size_mb">📦 {{ view.work.file_size_mb.toFixed(1) }}MB</span>
          <span v-if="view.work.age_class" class="uppercase">{{ view.work.age_class }}</span>
        </div>

        <div v-if="view.actors.length" class="mt-2 text-xs text-muted">
          🎤 声优：<span class="text-white/80">{{ view.actors.join(" / ") }}</span>
        </div>

        <!-- 标签 -->
        <div class="mt-3">
          <TagEditor :work-id="view.work.id" :tags="view.tags" @updated="onTagsUpdated" />
        </div>

        <!-- 操作按钮 -->
        <div class="flex flex-wrap gap-2 mt-5">
          <button class="btn-primary" @click="playAll" :disabled="!tracks.length">
            <Play :size="14" /> 播放
          </button>
          <button class="btn-outline" :disabled="downloading" @click="startDownload">
            <Loader2 v-if="downloading" :size="14" class="animate-spin" />
            <Download v-else :size="14" />
            {{ view.download_status === "downloaded" ? "重新下载" : "下载" }}
          </button>
          <button class="btn-outline" :disabled="scanningTracks" @click="scanTracks">
            <Loader2 v-if="scanningTracks" :size="14" class="animate-spin" />
            <Music v-else :size="14" />
            扫描音轨
          </button>
          <button class="btn-ghost" @click="openLocal">
            <FolderOpen :size="14" /> 本地文件夹
          </button>
          <button class="btn-ghost" @click="bindLocal" title="选择本地已有文件夹并自动匹配分组与音轨">
            <FolderPlus :size="14" /> 关联本地
          </button>
          <button class="btn-outline" :disabled="translating" title="用 LLM 把日文标题/简介翻译成简体中文" @click="translateWork">
            <Loader2 v-if="translating" :size="14" class="animate-spin" />
            <Languages v-else :size="14" />
            LLM 翻译
          </button>
          <button class="btn-ghost" @click="openDlsite">
            <Globe :size="14" /> DLsite 页面
          </button>
          <button class="btn-ghost" @click="openAsmrOne">
            <Monitor :size="14" /> ASMR One
          </button>
          <button class="btn-outline" :disabled="onlineLoading" @click="playOnline">
            <Loader2 v-if="onlineLoading" :size="14" class="animate-spin" />
            <Headphones v-else :size="14" />
            asmr.one 在线试听
          </button>
          <button class="btn-outline !border-accent/40 !text-accent-light hover:!bg-accent/10" @click="openAsmrDownload">
            <Download :size="14" />
            asmr.one 下载
          </button>
          <button class="btn-ghost !text-accent hover:!bg-accent/10" @click="deleteWork">
            <Trash2 :size="14" /> 删除
          </button>
        </div>

        <!-- 下载链接输入 -->
        <div v-if="showUrlInput" class="mt-3 animate-fade-in">
          <div class="text-[11px] text-muted mb-1.5">
            🔗 输入压缩包直链（在 DLsite 购买页面登录后复制下载链接，支持 zip 自动解压）：
          </div>
          <div class="flex gap-2">
            <input
              v-model="dlUrl"
              class="input flex-1 !text-xs"
              placeholder="https://www.dlsite.com/.../download.zip"
              @keyup.enter="startDownload"
            />
            <button class="btn-primary !text-xs" :disabled="downloading" @click="startDownload">
              开始下载
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 简介（LLM 翻译结果优先） -->
    <div v-if="view.work.description_zh || view.work.description" class="card p-4">
      <div class="text-xs font-semibold text-white mb-2 flex items-center gap-2">
        📝 简介
        <span
          v-if="view.work.description_zh"
          class="px-1.5 py-0.5 rounded text-[9px] bg-accent/15 text-accent-light font-normal"
        >中文翻译</span>
      </div>
      <p class="text-xs text-muted leading-relaxed whitespace-pre-line max-h-48 overflow-y-auto">
        {{ view.work.description_zh || view.work.description }}
      </p>
    </div>

    <!-- 音轨 -->
    <TrackList :tracks="tracks" :work="view" />

    <!-- asmr.one 文件树下载选择器 Modal -->
    <AsmrFileTree
      :visible="showFileTree"
      :nodes="treeNodes"
      :loading="treeLoading"
      :rj-code="view.work.rj_code"
      @close="showFileTree = false"
      @download="handleAsmrDownload"
    />

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      title="选择该作品的本地文件夹（服务器上的目录）"
      @close="showFolderPicker = false"
      @select="handleFolderPicked"
    />
  </div>
</template>
