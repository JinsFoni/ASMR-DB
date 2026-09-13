<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { X, Download, Loader2 } from "lucide-vue-next";
import type { AsmrTreeNode, AsmrDownloadFile } from "../lib/types";
import AsmrTreeNodeItem from "./AsmrTreeNodeItem.vue";

interface FlatFileItem {
  node: AsmrTreeNode;
  relativePath: string;
}

const props = defineProps<{
  visible: boolean;
  nodes: AsmrTreeNode[];
  loading: boolean;
  rjCode?: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "download", files: AsmrDownloadFile[]): void;
}>();

// 存储选中文件的唯一标识（用 relativePath 作为 key 确保唯一）
const selectedPaths = ref<Set<string>>(new Set());
// 折叠/展开的文件夹路径集合
const expandedFolders = ref<Set<string>>(new Set());

// 扁平化所有文件列表（带完整相对路径）
function flattenFiles(nodes: AsmrTreeNode[], parentPath = ""): FlatFileItem[] {
  let list: FlatFileItem[] = [];
  for (const node of nodes) {
    const currentPath = parentPath ? `${parentPath}/${node.title}` : node.title;
    if (node.nodeType === "folder" || (node.children && node.children.length > 0)) {
      list = list.concat(flattenFiles(node.children || [], currentPath));
    } else {
      list.push({
        node,
        relativePath: currentPath,
      });
    }
  }
  return list;
}

const allFiles = computed(() => flattenFiles(props.nodes));

// 自动默认全选音频和字幕
watch(
  () => props.nodes,
  (newNodes) => {
    if (newNodes && newNodes.length > 0) {
      // 默认展开顶层文件夹
      const newExp = new Set<string>();
      for (const n of newNodes) {
        if (n.nodeType === "folder" || (n.children && n.children.length > 0)) {
          newExp.add(n.title);
        }
      }
      expandedFolders.value = newExp;

      // 默认勾选音频和字幕
      const newSel = new Set<string>();
      const files = flattenFiles(newNodes);
      for (const item of files) {
        if (item.node.nodeType === "audio" || item.node.nodeType === "text") {
          newSel.add(item.relativePath);
        }
      }
      // 如果没有音频字幕，则全选
      if (newSel.size === 0) {
        for (const item of files) {
          newSel.add(item.relativePath);
        }
      }
      selectedPaths.value = newSel;
    }
  },
  { immediate: true }
);

// 统计信息
const selectedCount = computed(() => selectedPaths.value.size);

const selectedTotalSize = computed(() => {
  let bytes = 0;
  for (const item of allFiles.value) {
    if (selectedPaths.value.has(item.relativePath)) {
      bytes += item.node.size || 0;
    }
  }
  return bytes;
});

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "未知大小";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

// 快速选择动作
function selectByType(type: "audio" | "text" | "image" | "all" | "none") {
  if (type === "none") {
    selectedPaths.value = new Set();
    return;
  }
  if (type === "all") {
    const s = new Set<string>();
    for (const f of allFiles.value) s.add(f.relativePath);
    selectedPaths.value = s;
    return;
  }
  const s = new Set(selectedPaths.value);
  for (const f of allFiles.value) {
    if (f.node.nodeType === type) {
      s.add(f.relativePath);
    }
  }
  selectedPaths.value = s;
}

function toggleFolder(folderPath: string) {
  const s = new Set(expandedFolders.value);
  if (s.has(folderPath)) s.delete(folderPath);
  else s.add(folderPath);
  expandedFolders.value = s;
}

function expandAll() {
  const s = new Set<string>();
  function addFolders(nodes: AsmrTreeNode[], parent = "") {
    for (const n of nodes) {
      const p = parent ? `${parent}/${n.title}` : n.title;
      if (n.nodeType === "folder" || (n.children && n.children.length > 0)) {
        s.add(p);
        addFolders(n.children || [], p);
      }
    }
  }
  addFolders(props.nodes);
  expandedFolders.value = s;
}

function collapseAll() {
  expandedFolders.value = new Set();
}

function toggleFile(relativePath: string) {
  const s = new Set(selectedPaths.value);
  if (s.has(relativePath)) s.delete(relativePath);
  else s.add(relativePath);
  selectedPaths.value = s;
}

// 勾选/取消勾选某个文件夹下的所有文件
function toggleFolderFiles(node: AsmrTreeNode, parentPath: string) {
  const currentPath = parentPath ? `${parentPath}/${node.title}` : node.title;
  const folderFiles = flattenFiles(node.children || [], currentPath);
  const allChecked = folderFiles.every((f) => selectedPaths.value.has(f.relativePath));

  const s = new Set(selectedPaths.value);
  for (const f of folderFiles) {
    if (allChecked) s.delete(f.relativePath);
    else s.add(f.relativePath);
  }
  selectedPaths.value = s;
}

function handleConfirmDownload() {
  const result: AsmrDownloadFile[] = [];
  for (const item of allFiles.value) {
    if (selectedPaths.value.has(item.relativePath)) {
      result.push({
        title: item.node.title,
        downloadUrl: item.node.downloadUrl,
        relativePath: item.relativePath,
      });
    }
  }
  if (result.length === 0) return;
  emit("download", result);
}
</script>

<template>
  <div
    v-if="visible"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-fade-in"
  >
    <div
      class="bg-bg border border-bg-border w-full max-w-3xl max-h-[85vh] rounded-2xl flex flex-col shadow-2xl overflow-hidden"
    >
      <!-- 头部 -->
      <div class="px-5 py-4 border-b border-bg-border flex items-center justify-between shrink-0">
        <div class="flex items-center gap-2">
          <Download :size="18" class="text-accent" />
          <h2 class="text-sm font-semibold text-white">
            asmr.one 文件下载
            <span v-if="rjCode" class="text-xs font-mono text-muted ml-1">({{ rjCode }})</span>
          </h2>
        </div>
        <button
          class="text-muted hover:text-white p-1 rounded-lg hover:bg-bg-hover transition-colors"
          @click="emit('close')"
        >
          <X :size="16" />
        </button>
      </div>

      <!-- 加载中 -->
      <div v-if="loading" class="flex-1 flex flex-col items-center justify-center py-20 text-muted">
        <Loader2 :size="32" class="animate-spin text-accent mb-3" />
        <p class="text-xs">正在从 asmr.one 获取文件树目录...</p>
      </div>

      <!-- 内容区 -->
      <div v-else class="flex-1 flex flex-col min-h-0">
        <!-- 筛选与控制栏 -->
        <div class="px-5 py-2.5 bg-bg-hover/40 border-b border-bg-border flex flex-wrap items-center justify-between gap-2 shrink-0">
          <!-- 快捷筛选 -->
          <div class="flex flex-wrap items-center gap-1.5 text-xs">
            <span class="text-muted text-[11px] mr-1">快捷选择:</span>
            <button
              class="px-2 py-0.5 rounded border border-bg-border text-white/80 hover:text-white hover:bg-bg-hover hover:border-accent/40 transition-colors"
              @click="selectByType('audio')"
            >
              🎵 全选音频
            </button>
            <button
              class="px-2 py-0.5 rounded border border-bg-border text-white/80 hover:text-white hover:bg-bg-hover hover:border-accent/40 transition-colors"
              @click="selectByType('text')"
            >
              💬 全选字幕
            </button>
            <button
              class="px-2 py-0.5 rounded border border-bg-border text-white/80 hover:text-white hover:bg-bg-hover hover:border-accent/40 transition-colors"
              @click="selectByType('image')"
            >
              🖼️ 全选图片
            </button>
            <button
              class="px-2 py-0.5 rounded border border-bg-border text-white/80 hover:text-white hover:bg-bg-hover hover:border-accent/40 transition-colors"
              @click="selectByType('all')"
            >
              全选
            </button>
            <button
              class="px-2 py-0.5 rounded border border-bg-border text-muted hover:text-white hover:bg-bg-hover transition-colors"
              @click="selectByType('none')"
            >
              清空
            </button>
          </div>

          <!-- 展开/折叠全部 -->
          <div class="flex items-center gap-2 text-xs">
            <button class="text-[11px] text-muted hover:text-white" @click="expandAll">
              全部展开
            </button>
            <span class="text-bg-border">|</span>
            <button class="text-[11px] text-muted hover:text-white" @click="collapseAll">
              全部折叠
            </button>
          </div>
        </div>

        <!-- 树形节点展示区 -->
        <div class="flex-1 overflow-y-auto p-3 space-y-0.5 select-none font-sans text-xs">
          <template v-if="nodes.length === 0">
            <div class="py-12 text-center text-muted text-xs">该作品在 asmr.one 上未找到任何文件</div>
          </template>

          <template v-else>
            <!-- 递归渲染树组件 -->
            <AsmrTreeNodeItem
              v-for="(node, i) in nodes"
              :key="node.hash || `${node.title}_${i}`"
              :node="node"
              :parent-path="''"
              :selected-paths="selectedPaths"
              :expanded-folders="expandedFolders"
              @toggle-file="toggleFile"
              @toggle-folder="toggleFolder"
              @toggle-folder-files="toggleFolderFiles"
            />
          </template>
        </div>

        <!-- 底部操作栏 -->
        <div class="px-5 py-3 border-t border-bg-border bg-bg-hover/20 flex items-center justify-between shrink-0">
          <div class="text-xs text-muted flex items-center gap-2">
            <span>已选择 <b class="text-accent font-semibold">{{ selectedCount }}</b> / {{ allFiles.length }} 个文件</span>
            <span class="text-bg-border">|</span>
            <span>预估大小: <b class="text-white/90">{{ formatBytes(selectedTotalSize) }}</b></span>
          </div>

          <div class="flex items-center gap-2">
            <button class="btn-ghost !text-xs" @click="emit('close')">取消</button>
            <button
              class="btn-primary !text-xs flex items-center gap-1.5 shadow-lg shadow-accent/20"
              :disabled="selectedCount === 0"
              @click="handleConfirmDownload"
            >
              <Download :size="14" /> 开始下载 ({{ selectedCount }})
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
