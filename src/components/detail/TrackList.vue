<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { Play, Pause, Music, ExternalLink, Image as ImageIcon, FileText, File, Folder, ChevronDown, ChevronRight } from "lucide-vue-next";
import type { Track, WorkView, AsmrTreeNode } from "../../lib/types";
import { usePlayerStore } from "../../stores/player";
import * as api from "../../lib/api";

const props = defineProps<{
  tracks: Track[];
  work: WorkView;
  /** 在线音轨（asmr.one）：隐藏本地播放器按钮，调整空状态文案 */
  online?: boolean;
  /** asmr.one 完整文件树：提供时显示 音频/全部文件 切换 */
  tree?: AsmrTreeNode[] | null;
}>();
const player = usePlayerStore();
const router = useRouter();

const viewMode = ref<"audio" | "all">("audio");
watch(
  () => props.tree,
  () => {
    viewMode.value = "audio";
    expanded.value = new Set();
  }
);

/** 展开的文件夹（key 为路径），默认全部折叠，点击文件夹展开/收起 */
const expanded = ref<Set<string>>(new Set());
const toggleFolder = (key: string) => {
  const s = new Set(expanded.value);
  if (s.has(key)) s.delete(key);
  else s.add(key);
  expanded.value = s;
};

interface TreeRow {
  key: string;
  depth: number;
  title: string;
  nodeType: string;
  extension: string;
  size: number | null;
  duration: number | null;
  isFolder: boolean;
  fileCount: number;
}

/** 文件树 → 带缩进层级的展开行（可折叠文件夹） */
const treeRows = computed<TreeRow[]>(() => {
  const rows: TreeRow[] = [];
  const walk = (nodes: AsmrTreeNode[], depth: number, prefix: string) => {
    nodes.forEach((n, i) => {
      const isFolder = n.nodeType === "folder" || (n.children?.length ?? 0) > 0;
      const key = `${prefix}/${i}:${n.title}`;
      rows.push({
        key,
        depth,
        title: n.title,
        nodeType: n.nodeType,
        extension: n.extension,
        size: n.size,
        duration: n.duration,
        isFolder,
        fileCount: isFolder ? countTreeFiles(n.children || []) : 0,
      });
      if (isFolder && expanded.value.has(key)) walk(n.children || [], depth + 1, key);
    });
  };
  walk(props.tree || [], 0, "");
  return rows;
});

/** 全树文件总数（不受折叠状态影响） */
const totalFileCount = computed(() => countTreeFiles(props.tree || []));

/** 全树总大小（不受折叠状态影响） */
const totalSize = computed(() => {
  const sum = (nodes: AsmrTreeNode[]): number =>
    nodes.reduce((s, n) => s + (n.size || 0) + sum(n.children || []), 0);
  return sum(props.tree || []);
});

function countTreeFiles(nodes: AsmrTreeNode[]): number {
  return nodes.reduce(
    (s, n) =>
      s + (n.nodeType === "folder" || (n.children?.length ?? 0) > 0 ? countTreeFiles(n.children || []) : 1),
    0
  );
}

const fmtSize = (b: number | null) => {
  if (!b) return "--";
  if (b >= 1024 ** 3) return (b / 1024 ** 3).toFixed(2) + " GB";
  if (b >= 1024 ** 2) return (b / 1024 ** 2).toFixed(1) + " MB";
  return Math.max(1, Math.round(b / 1024)) + " KB";
};

const fileIcon = (t: string) =>
  t === "audio" ? Music : t === "image" ? ImageIcon : t === "text" ? FileText : File;

const typeLabel = (f: { nodeType: string; extension: string }) => {
  if (f.nodeType === "image") return "图片";
  if (f.nodeType === "text") return "字幕/文本";
  return f.extension ? f.extension.toUpperCase() : "文件";
};

const fmtDuration = (s: number | null) => {
  if (!s) return "--:--";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
};

const isCurrent = computed(() => (t: Track) =>
  player.work?.work.id === props.work.work.id && player.currentTrack?.id === t.id
);

async function openWithPlayer(t: Track) {
  try {
    await api.openAudioFile(t.file_path);
  } catch (e) {
    alert(`打开失败: ${e}`);
  }
}

/** 播放指定音轨并进入全屏播放器页面。 */
function playTrack(i: number) {
  player.playFromWork(props.work, props.tracks, i);
  router.push({ name: "player" });
}
</script>

<template>
  <div class="card overflow-hidden">
    <div class="px-4 py-2.5 border-b border-bg-border flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <!-- 音频/全部文件切换（提供文件树时显示） -->
        <div
          v-if="props.tree?.length"
          class="flex rounded-lg border border-bg-border overflow-hidden shrink-0"
        >
          <button
            class="px-2 py-1 text-[10px] transition-colors"
            :class="viewMode === 'audio' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white'"
            @click="viewMode = 'audio'"
          >
            音频 {{ tracks.length }}
          </button>
          <button
            class="px-2 py-1 text-[10px] transition-colors"
            :class="viewMode === 'all' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white'"
            @click="viewMode = 'all'"
          >
            全部文件 {{ totalFileCount }}
          </button>
        </div>
        <div class="text-xs font-semibold text-white">
          {{ viewMode === "all" ? `📁 文件结构 (${totalFileCount} 个文件)` : `🎵 音轨列表 (${tracks.length})` }}
        </div>
      </div>
      <span class="text-[10px] text-muted shrink-0">
        {{ viewMode === "all" ? `共 ${fmtSize(totalSize)}` : (props.work.total_duration_sec / 60).toFixed(0) + " 分钟" }}
      </span>
    </div>

    <!-- 全部文件视图：树形结构（文件夹可折叠，子级缩进） -->
    <div v-if="viewMode === 'all'" class="max-h-96 overflow-y-auto py-1">
      <div
        v-for="r in treeRows"
        :key="r.key"
        class="flex items-center gap-2 pr-4 py-1.5 hover:bg-bg-hover transition-colors"
        :class="r.isFolder ? 'cursor-pointer select-none' : ''"
        :style="{ paddingLeft: 16 + r.depth * 18 + 'px' }"
        :title="r.isFolder ? '点击折叠/展开' : undefined"
        @click="r.isFolder && toggleFolder(r.key)"
      >
        <template v-if="r.isFolder">
          <span class="w-4 flex items-center justify-center text-muted shrink-0">
            <ChevronDown v-if="expanded.has(r.key)" :size="12" />
            <ChevronRight v-else :size="12" />
          </span>
          <span class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-bg-hover text-muted">
            <Folder :size="12" />
          </span>
          <span class="text-xs text-white/90 ellipsis-1 flex-1 min-w-0">{{ r.title }}</span>
          <span class="text-[10px] text-muted/70 shrink-0">{{ r.fileCount }} 个文件</span>
        </template>
        <template v-else>
          <span class="w-4 shrink-0" />
          <span class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-bg-hover text-muted">
            <component :is="fileIcon(r.nodeType)" :size="11" />
          </span>
          <span class="text-xs text-white/80 ellipsis-1 flex-1 min-w-0">{{ r.title }}</span>
          <span class="text-[9px] px-1 py-0.5 rounded bg-bg-hover text-muted uppercase shrink-0">
            {{ typeLabel({ nodeType: r.nodeType, extension: r.extension } as any) }}
          </span>
          <span class="text-[10px] text-muted tabular-nums shrink-0 w-16 text-right">
            {{ fmtSize(r.size) }}
          </span>
        </template>
      </div>
      <div v-if="!treeRows.length" class="px-4 py-6 text-center text-xs text-muted">
        没有其他文件
      </div>
    </div>

    <!-- 音频视图 -->
    <div v-else class="max-h-96 overflow-y-auto">
      <div v-if="!tracks.length" class="px-4 py-6 text-center text-xs text-muted">
        {{ online ? "asmr.one 上未找到音频文件" : "暂无音轨。下载并解压作品后会自动扫描。" }}
      </div>
      <div
        v-for="(t, i) in tracks"
        :key="t.id"
        class="w-full flex items-center gap-3 px-4 py-2 text-left hover:bg-bg-hover transition-colors group"
        :class="isCurrent(t) ? 'bg-accent/10' : ''"
        @dblclick="playTrack(i)"
      >
        <span class="w-6 text-center text-[11px] font-mono text-muted shrink-0">
          {{ String(t.track_number || i + 1).padStart(2, "0") }}
        </span>
        <button
          class="w-6 h-6 rounded-full flex items-center justify-center shrink-0 cursor-pointer"
          :class="isCurrent(t) ? 'bg-accent text-white' : 'bg-bg-hover text-muted group-hover:bg-bg-border group-hover:text-white'"
          :title="isCurrent(t) ? '当前播放' : '播放'"
          @click="playTrack(i)"
        >
          <Music v-if="!isCurrent(t)" :size="12" />
          <Play v-else-if="!player.playing" :size="12" />
          <Pause v-else :size="12" />
        </button>
        <div class="flex-1 min-w-0 cursor-pointer" :title="'双击播放：' + (t.title || t.file_path.split(/[\\/]/).pop())" @dblclick="playTrack(i)">
          <div class="text-xs ellipsis-1" :class="isCurrent(t) ? 'text-accent-light' : 'text-white'">
            {{ t.title || t.file_path.split(/[\\/]/).pop() }}
          </div>
        </div>
        <button
          v-if="!props.online"
          class="shrink-0 px-1.5 py-0.5 rounded text-[10px] bg-bg-hover text-muted hover:text-white hover:bg-bg-border transition-colors"
          :title="'用本地播放器打开'"
          @click="openWithPlayer(t)"
        >
          <ExternalLink :size="11" /> 本地播放器
        </button>
        <span v-if="t.file_format" class="text-[9px] px-1 py-0.5 rounded bg-bg-hover text-muted uppercase">
          {{ t.file_format }}
        </span>
        <span class="text-[11px] text-muted tabular-nums shrink-0">
          {{ fmtDuration(t.duration_sec) }}
        </span>
      </div>
    </div>
  </div>
</template>
