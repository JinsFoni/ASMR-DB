<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { Music, ExternalLink, Image as ImageIcon, FileText, File, Folder, ChevronDown, ChevronRight } from "lucide-vue-next";
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
  <section class="card panel-tracks">
    <h2>
      音轨 <span class="n">{{ tracks.length }}</span>
      <template v-if="props.tree?.length">
        <span class="tr-switch">
          <button :class="{ on: viewMode === 'audio' }" @click="viewMode = 'audio'">仅音频</button>
          <button :class="{ on: viewMode === 'all' }" @click="viewMode = 'all'">全部文件 {{ totalFileCount }}</button>
        </span>
      </template>
      <span class="flex-1" />
      <span class="tr-sum">
        {{ viewMode === "all" ? `共 ${fmtSize(totalSize)}` : (props.work.total_duration_sec / 60).toFixed(0) + " 分钟" }}
      </span>
    </h2>

    <!-- 全部文件视图：树形结构（文件夹可折叠，子级缩进） -->
    <div v-if="viewMode === 'all'" class="tracks max-h-96 overflow-y-auto py-1">
      <div
        v-for="r in treeRows"
        :key="r.key"
        class="track"
        :class="r.isFolder ? 'cursor-pointer select-none' : ''"
        :style="{ paddingLeft: 12 + r.depth * 18 + 'px' }"
        :title="r.isFolder ? '点击折叠/展开' : undefined"
        @click="r.isFolder && toggleFolder(r.key)"
      >
        <template v-if="r.isFolder">
          <span class="no">
            <ChevronDown v-if="expanded.has(r.key)" :size="12" />
            <ChevronRight v-else :size="12" />
          </span>
          <span class="f-ico"><Folder :size="12" /></span>
          <span class="tname">{{ r.title }}</span>
          <span class="dur">{{ r.fileCount }} 个文件</span>
        </template>
        <template v-else>
          <span class="no"><component :is="fileIcon(r.nodeType)" :size="12" /></span>
          <span class="tname">{{ r.title }}</span>
          <span class="ftag">{{ typeLabel({ nodeType: r.nodeType, extension: r.extension } as any) }}</span>
          <span class="dur">{{ fmtSize(r.size) }}</span>
        </template>
      </div>
      <div v-if="!treeRows.length" class="px-4 py-6 text-center text-xs" style="color: var(--label-3)">
        没有其他文件
      </div>
    </div>

    <!-- 音频视图 -->
    <div v-else class="tracks max-h-96 overflow-y-auto">
      <div v-if="!tracks.length" class="px-4 py-6 text-center text-xs" style="color: var(--label-3)">
        {{ online ? "asmr.one 上未找到音频文件" : "暂无音轨。下载并解压作品后会自动扫描。" }}
      </div>
      <div
        v-for="(t, i) in tracks"
        :key="t.id"
        class="track"
        :class="{ playing: isCurrent(t) }"
        :title="'点击播放：' + (t.title || t.file_path.split(/[\\/]/).pop())"
        @click="playTrack(i)"
      >
        <!-- 播放中显示 EQ 动画（设计稿 .eq） -->
        <span v-if="isCurrent(t)" class="eq"><i /><i /><i /></span>
        <span v-else class="no">{{ String(t.track_number || i + 1).padStart(2, "0") }}</span>
        <span class="tname">{{ t.title || t.file_path.split(/[\\/]/).pop() }}</span>
        <button
          v-if="!props.online"
          class="op-local"
          title="用本地播放器打开"
          @click.stop="openWithPlayer(t)"
        >
          <ExternalLink :size="11" /> 本地播放器
        </button>
        <span class="dur">{{ fmtDuration(t.duration_sec) }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel-tracks {
  margin-top: 30px;
  padding: 26px 28px;
}
.panel-tracks h2 {
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--label);
}
.panel-tracks h2 .n {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--label-3);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 2px 10px;
}
.tr-switch {
  display: inline-flex;
  gap: 2px;
  background: var(--fill-1);
  border-radius: 999px;
  padding: 2px;
  margin-left: 4px;
}
.tr-switch button {
  height: 24px;
  padding: 0 11px;
  border: none;
  background: none;
  border-radius: 999px;
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  color: var(--label-2);
  cursor: pointer;
  transition: all 0.2s;
}
.tr-switch button.on {
  background: #48484c;
  color: #fff;
}
html[data-theme="light"] .tr-switch button.on,
html[data-theme="sakura"] .tr-switch button.on {
  background: var(--blue);
}
.tr-sum {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--label-3);
}

.tracks {
  display: flex;
  flex-direction: column;
}
.track {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 13px 12px;
  border-radius: var(--r-md);
  cursor: pointer;
  transition: background 0.18s;
}
.track:hover {
  background: var(--fill-1);
}
.track.playing {
  background: var(--fill-2);
}
.track .no {
  width: 26px;
  text-align: center;
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 12px;
  color: var(--label-3);
  flex-shrink: 0;
  display: grid;
  place-items: center;
}
.track .f-ico {
  width: 22px;
  height: 22px;
  border-radius: 7px;
  background: var(--fill-1);
  color: var(--label-2);
  display: grid;
  place-items: center;
  flex-shrink: 0;
}
.track .tname {
  flex: 1;
  min-width: 0;
  font-size: 13.5px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.track.playing .tname {
  color: var(--blue-deep);
}
.track .dur {
  font-size: 12px;
  color: var(--label-3);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}
.track .ftag {
  font-size: 9.5px;
  font-weight: 600;
  color: var(--label-3);
  background: var(--fill-1);
  border-radius: 5px;
  padding: 2px 7px;
  text-transform: uppercase;
  flex-shrink: 0;
}
.op-local {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--label-3);
  background: none;
  border: none;
  border-radius: 999px;
  padding: 4px 9px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  flex-shrink: 0;
}
.op-local:hover {
  background: var(--fill-2);
  color: var(--label);
}

/* 播放中 EQ 动画（设计稿 .track .eq） */
.track .eq {
  display: flex;
  align-items: flex-end;
  gap: 2.5px;
  height: 14px;
  width: 26px;
  justify-content: center;
  flex-shrink: 0;
}
.track .eq i {
  width: 3px;
  border-radius: 2px;
  background: var(--blue-deep);
  animation: eq 1s ease-in-out infinite;
}
.track .eq i:nth-child(2) {
  animation-delay: 0.25s;
}
.track .eq i:nth-child(3) {
  animation-delay: 0.5s;
}
@keyframes eq {
  0%,
  100% {
    height: 5px;
  }
  50% {
    height: 14px;
  }
}
</style>
