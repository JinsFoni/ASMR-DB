<script setup lang="ts">
import { computed } from "vue";
import {
  ArrowLeft,
  BookmarkCheck,
  Check,
  Copy,
  Download,
  FolderPlus,
  Globe,
  Headphones,
  ImageOff,
  Languages,
  Library,
  Loader2,
  Monitor,
  Music,
  Play,
  RotateCw,
  Trash2,
} from "lucide-vue-next";
import type { AsmrDownloadFile, AsmrTreeNode, Tag, Track, WorkView } from "../../lib/types";
import type { ActionKey, DetailAction, MetaItem, StatusBadge } from "../../lib/workDetail";
import TrackList from "./TrackList.vue";
import TagEditor from "./TagEditor.vue";
import AsmrFileTree from "../AsmrFileTree.vue";
import FolderPickerModal from "../common/FolderPickerModal.vue";

const props = defineProps<{
  loading: boolean;
  error: string;
  title: string;
  subtitle?: string | null;
  rjCode?: string | null;
  coverUrl?: string | null;
  statusBadge?: StatusBadge | null;
  meta: MetaItem[];
  actors: string[];
  tags: Tag[];
  editableTags: boolean;
  workId?: number | null;
  description?: string | null;
  descriptionNote?: string | null;
  descriptionLoading?: boolean;
  actions: DetailAction[];
  tracks: Track[];
  trackWork: WorkView | null;
  onlineTracks?: boolean;
  tree?: AsmrTreeNode[] | null;
  tracksLoading?: boolean;
  treeLoading?: boolean;
  /** 主数据不存在（作品不存在或已被删除） */
  notFound?: boolean;
  busyKey?: ActionKey | null;
  copied?: boolean;
  fileTreeVisible?: boolean;
  folderPickerVisible?: boolean;
  /** DLsite 压缩包直链输入框（仅作品库「下载」使用） */
  urlInputVisible?: boolean;
  urlInputValue?: string;
  /** 错误态下的替代外链（DLsite 页用 DLsite，其余用 ASMR ONE） */
  errorLinkLabel?: string;
  retryLabel?: string;
}>();

const emit = defineEmits<{
  (e: "retry"): void;
  (e: "tags-updated"): void;
  (e: "back"): void;
  (e: "action", key: ActionKey): void;
  (e: "file-tree-download", files: AsmrDownloadFile[]): void;
  (e: "folder-picked", dir: string): void;
  (e: "update:fileTreeVisible", v: boolean): void;
  (e: "update:folderPickerVisible", v: boolean): void;
  (e: "update:urlInputVisible", v: boolean): void;
  (e: "update:urlInputValue", v: string): void;
  (e: "submit-url"): void;
  (e: "copy-rj"): void;
  (e: "error-link"): void;
}>();

const ICONS: Record<ActionKey, unknown> = {
  "play-all": Play,
  "online-play": Headphones,
  import: BookmarkCheck,
  "go-local": Library,
  "asmr-download": Download,
  "dl-download": Download,
  "scan-tracks": Music,
  "bind-local": FolderPlus,
  translate: Languages,
  delete: Trash2,
  "open-dlsite": Globe,
  "open-asmr": Monitor,
};

function iconOf(key: ActionKey) {
  return ICONS[key] as never;
}

/** 只有会产生网络请求的动作才显示 spinner */
const SPINNER_KEYS: ActionKey[] = [
  "online-play",
  "import",
  "asmr-download",
  "dl-download",
  "scan-tracks",
  "translate",
];

const statusClass = computed(() => {
  switch (props.statusBadge?.tone) {
    case "green":
      return "border-green-500/50 text-green-400 bg-green-500/10";
    case "accent":
      return "border-accent/50 text-accent-light bg-accent/10";
    case "yellow":
      return "border-yellow-500/50 text-yellow-400 bg-yellow-500/10";
    default:
      return "border-bg-border text-muted bg-bg-hover";
  }
});
</script>

<template>
  <div v-if="loading" class="flex items-center justify-center py-24 text-muted">
    <Loader2 :size="24" class="animate-spin" />
  </div>

  <div v-else-if="error" class="p-5 max-w-5xl mx-auto space-y-4">
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="emit('back')">
      <ArrowLeft :size="14" /> 返回
    </button>
    <div class="card p-6 text-sm text-muted space-y-3">
      <div>⚠️ {{ error }}</div>
      <div class="flex gap-2">
        <button class="btn-outline !text-xs" :disabled="loading" @click="emit('retry')">
          <RotateCw :size="13" /> {{ retryLabel || "重试" }}
        </button>
        <button class="btn-ghost !text-xs" @click="emit('error-link')">
          <Globe :size="13" /> {{ errorLinkLabel || "打开 ASMR ONE 网页" }}
        </button>
      </div>
    </div>
  </div>

  <div v-else-if="notFound" class="p-5 text-center text-muted">作品不存在或已被删除</div>

  <div v-else class="p-5 space-y-5 max-w-5xl mx-auto">
    <!-- 返回 -->
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="emit('back')">
      <ArrowLeft :size="14" /> 返回
    </button>

    <div class="flex gap-5">
      <!-- 封面（完整封面不裁切，与横版卡片一致；加宽并垂直居中避免信息列下方大片空白） -->
      <div class="w-72 shrink-0 self-center rounded-xl overflow-hidden bg-bg-hover border border-bg-border">
        <img v-if="coverUrl" :src="coverUrl" class="w-full h-auto" decoding="async" />
        <div v-else class="w-72 aspect-[4/3] flex items-center justify-center text-muted/40">
          <ImageOff :size="40" />
        </div>
      </div>

      <!-- 信息 -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 text-xs">
          <template v-if="rjCode">
            <span class="font-mono text-muted">{{ rjCode }}</span>
            <button class="text-muted hover:text-white" title="复制RJ号" @click="emit('copy-rj')">
              <Check v-if="copied" :size="13" class="text-green-400" />
              <Copy v-else :size="13" />
            </button>
          </template>
          <span
            v-if="statusBadge"
            class="px-1.5 py-0.5 rounded text-[10px] border flex items-center gap-1"
            :class="[statusClass, statusBadge.clickable ? 'cursor-pointer' : '']"
            :title="statusBadge.clickable ? '该作品已在作品库中，点击查看本地详情' : undefined"
            @click="statusBadge.clickable && emit('action', 'go-local')"
          >
            <BookmarkCheck v-if="statusBadge.clickable" :size="10" />
            {{ statusBadge.text }}
          </span>
        </div>

        <h1 class="text-xl font-bold text-white mt-2 leading-snug">{{ title }}</h1>
        <div v-if="subtitle" class="text-sm text-muted mt-0.5">{{ subtitle }}</div>

        <!-- 元数据行 -->
        <div v-if="meta.length" class="flex flex-wrap gap-x-5 gap-y-1.5 mt-4 text-xs text-muted">
          <span v-for="(m, i) in meta" :key="i" :class="m.icon ? '' : 'uppercase'">
            <template v-if="m.icon">{{ m.icon }} </template>{{ m.text }}
          </span>
        </div>

        <div v-if="actors.length" class="mt-2 text-xs text-muted">
          🎤 声优：<span class="text-white/80">{{ actors.join(" / ") }}</span>
        </div>

        <!-- 标签：作品库可编辑，在线只读 -->
        <div v-if="editableTags && workId != null" class="mt-3">
          <TagEditor :work-id="workId" :tags="tags" @updated="emit('tags-updated')" />
        </div>
        <div v-else-if="tags.length" class="flex flex-wrap gap-1.5 mt-3">
          <span
            v-for="t in tags"
            :key="t.id"
            class="px-2 py-0.5 rounded text-[11px] bg-bg-hover text-muted border border-bg-border"
          >
            {{ t.name }}
          </span>
        </div>

        <!-- 操作按钮：全部由 actions 数组驱动 -->
        <div class="flex flex-wrap gap-2 mt-5">
          <button
            v-for="a in actions"
            :key="a.key"
            :class="{
              'btn-primary': a.variant === 'primary',
              'btn-outline': a.variant === 'outline',
              'btn-ghost': a.variant === 'ghost',
              '!border-accent/40 !text-accent-light hover:!bg-accent/10': a.key === 'asmr-download',
              '!text-accent hover:!bg-accent/10': a.key === 'delete',
            }"
            :disabled="busyKey === a.key"
            @click="emit('action', a.key)"
          >
            <Loader2
              v-if="busyKey === a.key && SPINNER_KEYS.includes(a.key)"
              :size="14"
              class="animate-spin"
            />
            <component :is="iconOf(a.key)" v-else :size="14" />
            {{ a.label }}
          </button>
        </div>
      </div>
    </div>

    <!-- DLsite 压缩包直链输入（作品库「下载」） -->
    <div v-if="urlInputVisible" class="animate-fade-in -mt-2">
      <div class="text-[11px] text-muted mb-1.5">
        🔗 输入压缩包直链（在 DLsite 购买页面登录后复制下载链接，支持 zip 自动解压）：
      </div>
      <div class="flex gap-2">
        <input
          :value="urlInputValue"
          class="input flex-1 !text-xs"
          placeholder="https://www.dlsite.com/.../download.zip"
          @input="emit('update:urlInputValue', ($event.target as HTMLInputElement).value)"
          @keyup.enter="emit('submit-url')"
        />
        <button class="btn-primary !text-xs" :disabled="busyKey === 'dl-download'" @click="emit('submit-url')">
          开始下载
        </button>
      </div>
    </div>

    <!-- 简介 -->
    <div v-if="description || descriptionLoading" class="card p-4">
      <div class="text-xs font-semibold text-white mb-2 flex items-center gap-2">
        📝 简介
        <span
          v-if="descriptionNote === '中文翻译'"
          class="px-1.5 py-0.5 rounded text-[9px] bg-accent/15 text-accent-light font-normal"
        >中文翻译</span>
        <span v-else-if="descriptionNote" class="text-[10px] font-normal text-muted">{{ descriptionNote }}</span>
      </div>
      <p v-if="descriptionLoading && !description" class="text-xs text-muted flex items-center gap-2">
        <Loader2 :size="14" class="animate-spin" /> 正在获取 DLsite 简介…
      </p>
      <p v-else class="text-xs text-muted leading-relaxed whitespace-pre-line max-h-48 overflow-y-auto">
        {{ description }}
      </p>
    </div>

    <!-- 音轨区：loading 仅由音轨拉取控制；无音轨卡片时按需展示空状态 -->
    <div v-if="tracksLoading" class="card p-6 flex items-center justify-center text-muted">
      <Loader2 :size="18" class="animate-spin" />
      <span class="text-xs ml-2">正在获取音轨列表…</span>
    </div>
    <TrackList
      v-else-if="trackWork"
      :tracks="tracks"
      :work="trackWork"
      :online="onlineTracks"
      :tree="tree"
    />

    <!-- asmr.one 文件树下载选择器 -->
    <AsmrFileTree
      :visible="fileTreeVisible ?? false"
      :nodes="tree ?? []"
      :loading="treeLoading ?? false"
      :rj-code="rjCode ?? undefined"
      @close="emit('update:fileTreeVisible', false)"
      @download="(files) => emit('file-tree-download', files)"
    />

    <!-- 服务端目录选择器 -->
    <FolderPickerModal
      :visible="folderPickerVisible ?? false"
      title="选择该作品的本地文件夹（服务器上的目录）"
      @close="emit('update:folderPickerVisible', false)"
      @select="(dir) => emit('folder-picked', dir)"
    />
  </div>
</template>