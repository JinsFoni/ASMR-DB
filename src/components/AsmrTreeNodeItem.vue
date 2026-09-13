<script setup lang="ts">
import { computed } from "vue";
import {
  Folder,
  FolderOpen,
  Music,
  FileText,
  Image as ImageIcon,
  File,
  ChevronRight,
  ChevronDown,
} from "lucide-vue-next";
import type { AsmrTreeNode } from "../lib/types";

const props = withDefaults(
  defineProps<{
    node: AsmrTreeNode;
    parentPath?: string;
    selectedPaths: Set<string>;
    expandedFolders: Set<string>;
    depth?: number;
  }>(),
  {
    parentPath: "",
    depth: 0,
  }
);

const emit = defineEmits<{
  (e: "toggleFile", path: string): void;
  (e: "toggleFolder", path: string): void;
  (e: "toggleFolderFiles", node: AsmrTreeNode, parentPath: string): void;
}>();

const isFolder = computed(
  () =>
    props.node.nodeType === "folder" ||
    (props.node.children && props.node.children.length > 0)
);

const currentPath = computed(() =>
  props.parentPath ? `${props.parentPath}/${props.node.title}` : props.node.title
);

const isExpanded = computed(() =>
  props.expandedFolders.has(currentPath.value)
);

function formatNodeSize(bytes: number | null): string {
  if (!bytes || bytes <= 0) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getAllDescendantFiles(node: AsmrTreeNode, p: string): string[] {
  let res: string[] = [];
  const cp = p ? `${p}/${node.title}` : node.title;
  if (node.nodeType === "folder" || (node.children && node.children.length > 0)) {
    for (const child of node.children || []) {
      res = res.concat(getAllDescendantFiles(child, cp));
    }
  } else {
    res.push(cp);
  }
  return res;
}

const folderCheckState = computed(() => {
  if (!isFolder.value) return { checked: false, indeterminate: false };
  const allFiles = getAllDescendantFiles(props.node, props.parentPath);
  if (allFiles.length === 0) return { checked: false, indeterminate: false };
  const count = allFiles.filter((f) => props.selectedPaths.has(f)).length;
  return {
    checked: count === allFiles.length,
    indeterminate: count > 0 && count < allFiles.length,
  };
});

const isFileChecked = computed(() =>
  props.selectedPaths.has(currentPath.value)
);
</script>

<template>
  <div class="w-full">
    <!-- 文件夹节点 -->
    <div
      v-if="isFolder"
      class="flex items-center gap-1.5 py-1 px-2 rounded-lg hover:bg-bg-hover/60 cursor-pointer transition-colors group"
      :style="{ paddingLeft: depth * 16 + 8 + 'px' }"
    >
      <!-- 折叠展开箭头 -->
      <button
        type="button"
        class="p-0.5 text-muted hover:text-white rounded"
        @click.stop="emit('toggleFolder', currentPath)"
      >
        <ChevronDown v-if="isExpanded" :size="14" />
        <ChevronRight v-else :size="14" />
      </button>

      <!-- 文件夹勾选框 -->
      <input
        type="checkbox"
        :checked="folderCheckState.checked"
        :indeterminate.prop="folderCheckState.indeterminate"
        class="rounded border-bg-border bg-bg-hover text-accent focus:ring-0 cursor-pointer w-3.5 h-3.5"
        @click.stop="emit('toggleFolderFiles', node, parentPath)"
      />

      <!-- 图标 + 文件夹名 -->
      <div class="flex items-center gap-1.5 flex-1 min-w-0" @click="emit('toggleFolder', currentPath)">
        <FolderOpen v-if="isExpanded" :size="15" class="text-amber-400 shrink-0" />
        <Folder v-else :size="15" class="text-amber-400 shrink-0" />
        <span class="text-white/90 font-medium truncate">{{ node.title }}</span>
        <span class="text-[10px] text-muted shrink-0">({{ node.children?.length || 0 }})</span>
      </div>
    </div>

    <!-- 文件节点 -->
    <div
      v-else
      class="flex items-center gap-2 py-1 px-2 rounded-lg hover:bg-bg-hover/60 cursor-pointer transition-colors"
      :style="{ paddingLeft: depth * 16 + 26 + 'px' }"
      @click="emit('toggleFile', currentPath)"
    >
      <input
        type="checkbox"
        :checked="isFileChecked"
        class="rounded border-bg-border bg-bg-hover text-accent focus:ring-0 cursor-pointer w-3.5 h-3.5"
        @click.stop="emit('toggleFile', currentPath)"
      />

      <!-- 图标 -->
      <Music v-if="node.nodeType === 'audio'" :size="14" class="text-accent-light shrink-0" />
      <FileText v-else-if="node.nodeType === 'text'" :size="14" class="text-emerald-400 shrink-0" />
      <ImageIcon v-else-if="node.nodeType === 'image'" :size="14" class="text-purple-400 shrink-0" />
      <File v-else :size="14" class="text-muted shrink-0" />

      <!-- 文件名 -->
      <span
        class="flex-1 truncate"
        :class="isFileChecked ? 'text-white' : 'text-muted hover:text-white/80'"
      >
        {{ node.title }}
      </span>

      <!-- 文件大小 / 时长 -->
      <span v-if="node.size" class="text-[10px] text-muted/70 font-mono shrink-0">
        {{ formatNodeSize(node.size) }}
      </span>
    </div>

    <!-- 子文件夹与文件递归 -->
    <div v-if="isFolder && isExpanded && node.children && node.children.length > 0">
      <AsmrTreeNodeItem
        v-for="(child, idx) in node.children"
        :key="child.hash || child.title + '_' + idx"
        :node="child"
        :parent-path="currentPath"
        :selected-paths="selectedPaths"
        :expanded-folders="expandedFolders"
        :depth="depth + 1"
        @toggle-file="(p) => emit('toggleFile', p)"
        @toggle-folder="(p) => emit('toggleFolder', p)"
        @toggle-folder-files="(n, p) => emit('toggleFolderFiles', n, p)"
      />
    </div>
  </div>
</template>
