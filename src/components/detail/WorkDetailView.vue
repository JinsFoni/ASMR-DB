<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ArrowLeft,
  BookmarkCheck,
  Check,
  Copy,
  Download,
  FolderPlus,
  Globe,
  Headphones,
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
  /** 年龄指定角标（R-18G / R18 / 全年龄），缺失时不渲染 */
  ageBadge?: string | null;
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

/* 封面加载失败（asmr.one 部分作品源站没有封面图，代理返回 502）时用默认图 */
const coverFailed = ref(false);
const DEFAULT_COVER = "/default-cover.png";

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
      return "badge green";
    case "accent":
      return "badge blue";
    case "yellow":
      return "badge amber";
    default:
      return "badge gray";
  }
});

/** 设计稿 meta-grid：label/value 两段式（icon 仅为键名提示，不入格） */
const metaCells = computed(() =>
  props.meta
    .filter((m) => m.text.includes("："))
    .map((m) => {
      const [label, ...rest] = m.text.split("：");
      return { label, value: rest.join("：") };
    })
);

/** 按设计稿：primary 主按钮 / 普通填充按钮 / ghost 描边 / danger 删除 */
function actionClass(a: DetailAction) {
  if (a.variant === "primary") return "btn primary";
  if (a.key === "delete") return "btn danger";
  if (a.variant === "ghost") return "btn ghost";
  return "btn";
}
</script>

<template>
  <div v-if="loading" class="flex items-center justify-center py-24 text-muted">
    <Loader2 :size="24" class="animate-spin" />
  </div>

  <div v-else-if="error" class="page-wrap">
    <button class="back-link" @click="emit('back')">
      <ArrowLeft :size="13" /> 返回
    </button>
    <div class="card p-6 text-sm space-y-3" style="color: var(--label-2)">
      <div>⚠️ {{ error }}</div>
      <div class="flex gap-2">
        <button class="btn ghost sm" :disabled="loading" @click="emit('retry')">
          <RotateCw :size="13" /> {{ retryLabel || "重试" }}
        </button>
        <button class="btn ghost sm" @click="emit('error-link')">
          <Globe :size="13" /> {{ errorLinkLabel || "打开 ASMR ONE 网页" }}
        </button>
      </div>
    </div>
  </div>

  <div v-else-if="notFound" class="p-5 text-center text-muted">作品不存在或已被删除</div>

  <div v-else class="page-wrap">
    <!-- 返回 -->
    <button class="back-link" @click="emit('back')">
      <ArrowLeft :size="13" /> 返回
    </button>

    <!-- 双列：封面 + 信息 -->
    <div class="detail">
      <div class="dcover">
        <img v-if="coverUrl && !coverFailed" :src="coverUrl" decoding="async" @error="coverFailed = true" />
        <img v-else :src="DEFAULT_COVER" decoding="async" class="default-cover" />
      </div>

      <div class="dtitle min-w-0">
        <h1>{{ title }}</h1>
        <p v-if="subtitle" class="ja">{{ subtitle }}</p>

        <div class="idrow">
          <template v-if="rjCode">
            <span class="rjid">{{ rjCode }}</span>
            <button class="copy-btn" title="复制RJ号" @click="emit('copy-rj')">
              <Check v-if="copied" :size="11" style="color: var(--green)" />
              <Copy v-else :size="11" />
              {{ copied ? "已复制" : "复制 RJ 号" }}
            </button>
          </template>
          <span
            v-if="statusBadge"
            :class="[statusClass, statusBadge.clickable ? 'cursor-pointer' : '']"
            :title="statusBadge.clickable ? '该作品已在作品库中，点击查看本地详情' : undefined"
            @click="statusBadge.clickable && emit('action', 'go-local')"
          >
            <BookmarkCheck v-if="statusBadge.clickable" :size="10" />
            {{ statusBadge.text }}
          </span>
          <span v-if="ageBadge" class="r18-tag">{{ ageBadge }}</span>
        </div>

        <!-- 元数据方格：两列（售出/收藏、发售/评分、社团/声优） -->
        <div v-if="metaCells.length || actors.length" class="meta-grid">
          <div v-for="(c, i) in metaCells" :key="'m' + i" class="meta-cell">
            <b>{{ c.label }}</b><span class="names" :title="c.value">{{ c.value }}</span>
          </div>
          <div v-if="actors.length" class="meta-cell">
            <b>声优</b><span class="names">{{ actors.join(" / ") }}</span>
          </div>
        </div>

        <!-- 标签：作品库可编辑，在线只读 -->
        <div v-if="editableTags && workId != null" class="tagrow">
          <TagEditor :work-id="workId" :tags="tags" @updated="emit('tags-updated')" />
        </div>
        <div v-else-if="tags.length" class="tagrow">
          <span v-for="t in tags" :key="t.id" class="btag">{{ t.name }}</span>
        </div>

        <!-- 操作按钮：全部由 actions 数组驱动 -->
        <div class="actions">
          <button
            v-for="a in actions"
            :key="a.key"
            :class="actionClass(a)"
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

        <!-- DLsite 压缩包直链输入（作品库「下载」） -->
        <div v-if="urlInputVisible" class="card dl-card animate-fade-in">
          <label class="field">
            <Download :size="14" />
            <input
              :value="urlInputValue"
              placeholder="粘贴下载 URL，点击「下载」开始（支持 zip 自动解压）"
              @input="emit('update:urlInputValue', ($event.target as HTMLInputElement).value)"
              @keyup.enter="emit('submit-url')"
            />
          </label>
          <button class="btn primary sm" style="height: 40px" :disabled="busyKey === 'dl-download'" @click="emit('submit-url')">
            下载
          </button>
        </div>
      </div>
    </div>

    <!-- 简介 -->
    <section v-if="description || descriptionLoading" class="card panel">
      <h2>
        简介
        <span v-if="descriptionNote === '中文翻译'" class="btag zh">中文翻译</span>
        <span v-else-if="descriptionNote" class="n">{{ descriptionNote }}</span>
      </h2>
      <p v-if="descriptionLoading && !description" class="desc flex items-center gap-2">
        <Loader2 :size="14" class="animate-spin" /> 正在获取 DLsite 简介…
      </p>
      <p v-else class="desc">{{ description }}</p>
    </section>

    <!-- 音轨区：loading 仅由音轨拉取控制；无音轨卡片时按需展示空状态 -->
    <section v-if="tracksLoading" class="card panel flex items-center justify-center" style="padding: 34px">
      <Loader2 :size="18" class="animate-spin" />
      <span class="text-xs ml-2" style="color: var(--label-2)">正在获取音轨列表…</span>
    </section>
    <section v-else-if="trackWork" class="mt-7">
      <TrackList
        :tracks="tracks"
        :work="trackWork"
        :online="onlineTracks"
        :tree="tree"
      />
    </section>

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

<style scoped>
.page-wrap {
  max-width: 1024px;
  margin: 0 auto;
  padding: 26px 28px 150px;
}
@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}

/* 返回胶囊 */
.back-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  color: var(--label-2);
  background: none;
  border: none;
  cursor: pointer;
  padding: 8px 14px;
  margin-left: -14px;
  border-radius: 999px;
  transition: background 0.2s, color 0.2s;
}
.back-link:hover {
  background: var(--fill-1);
  color: var(--label);
}

/* 双列布局 */
.detail {
  display: grid;
  grid-template-columns: 340px 1fr;
  gap: 36px;
  margin-top: 14px;
  align-items: start;
}
.dcover {
  aspect-ratio: 4 / 3;
  border-radius: var(--r-xl);
  overflow: hidden;
  background: var(--fill-1);
  box-shadow: var(--shadow-float), inset 0 0 0 0.5px rgba(255, 255, 255, 0.1);
}
.dcover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
@media (max-width: 900px) {
  .detail {
    grid-template-columns: 1fr;
  }
  .dcover {
    max-width: 300px;
  }
}

.dtitle h1 {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.32;
  color: var(--label);
}
.dtitle .ja {
  font-size: 13.5px;
  color: var(--label-2);
  margin-top: 8px;
  line-height: 1.55;
}

.idrow {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 14px;
  flex-wrap: wrap;
}
.rjid {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 12px;
  font-weight: 600;
  color: var(--label-2);
  background: var(--fill-1);
  border-radius: 8px;
  padding: 4px 10px;
}
.copy-btn {
  height: 26px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 11px;
  border: none;
  border-radius: 999px;
  background: var(--fill-2);
  color: var(--label-2);
  font-family: inherit;
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}
.copy-btn:hover {
  background: var(--fill-3);
  color: var(--label);
}

/* 元数据方格：两列，行优先（售出/收藏、发售/评分、社团/声优）
   第一列 240px 固定：让第二列起点与上方年龄指定角标对齐 */
.meta-grid {
  margin-top: 18px;
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  column-gap: 28px;
  row-gap: 9px;
  max-width: 560px;
}
.meta-cell {
  display: flex;
  align-items: baseline;
  gap: 7px;
  min-width: 0;
}
.meta-cell b {
  font-size: 12px;
  color: var(--label-3);
  font-weight: 600;
  flex-shrink: 0;
}
.meta-cell .names {
  font-size: 13px;
  font-weight: 600;
  color: var(--label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tagrow {
  display: flex;
  gap: 7px;
  flex-wrap: wrap;
  margin-top: 18px;
  align-items: center;
}
.btag {
  font-size: 12px;
  font-weight: 500;
  color: var(--label-2);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 5px 13px;
}
.btag.zh {
  color: #6cb1f7;
  background: rgba(47, 143, 239, 0.13);
}

.actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin-top: 24px;
}
.actions .btn {
  height: 42px;
  padding: 0 22px;
  font-size: 13.5px;
}

/* 下载 URL 卡 */
.dl-card {
  margin-top: 18px;
  padding: 14px 16px;
  display: flex;
  gap: 10px;
  align-items: center;
}
.dl-card .field {
  flex: 1;
  min-width: 200px;
  height: 40px;
  box-shadow: none;
  background: var(--fill-1);
}
@media (max-width: 640px) {
  .dl-card {
    flex-direction: column;
    align-items: stretch;
  }
}

/* 面板卡片（简介 / 音轨） */
.panel {
  margin-top: 30px;
  padding: 26px 28px;
}
.panel h2 {
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin-bottom: 14px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--label);
}
.panel h2 .n {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--label-3);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 2px 10px;
}
.desc {
  font-size: 14px;
  color: var(--label-2);
  line-height: 1.85;
  white-space: pre-line;
  max-height: 220px;
  overflow-y: auto;
}

/* 只读标签复用 TagEditor 内间距对齐 */
.tagrow :deep(.wtag) {
  font-size: 12px;
  font-weight: 500;
  color: var(--label-2);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 5px 13px;
}
</style>
