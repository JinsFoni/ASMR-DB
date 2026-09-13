<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import {
  Plus,
  FolderOpen,
  Loader2,
  Search,
  CheckCircle2,
  XCircle,
  Download,
} from "lucide-vue-next";
import * as api from "../lib/api";
import type { ScrapedWork, ScannedItem } from "../lib/types";
import FolderPickerModal from "../components/common/FolderPickerModal.vue";
import { useWorksStore } from "../stores/works";

const router = useRouter();
const worksStore = useWorksStore();

// ============ RJ 导入 ============
const rjInput = ref("");
const source = ref<"auto" | "dlsite" | "asmrone">("auto"); // 数据源
const previewing = ref(false);
const preview = ref<ScrapedWork | null>(null);
const previewError = ref<string | null>(null);
const importing = ref(false);
const importDone = ref<number | null>(null);
const importFeedback = ref<string | null>(null);

// 导入时可选关联本地文件夹
const rjLocalFolder = ref("");
const rjSmartGroup = ref(true);
const rjGroupName = ref("");

const sources = [
  { value: "auto", label: "智能自动", desc: "优先 DLsite，受限自动回退 asmr.one" },
  { value: "dlsite", label: "DLsite", desc: "官方直连" },
  { value: "asmrone", label: "asmr.one", desc: "asmr.one 数据源" },
];

// 目录选择器（服务端目录浏览）：rj = RJ 导入时关联本地文件夹，scan = 扫描根目录
const showFolderPicker = ref(false);
const pickerMode = ref<"rj" | "scan">("scan");

function pickRjLocalFolder() {
  pickerMode.value = "rj";
  showFolderPicker.value = true;
}

function handleFolderPicked(dir: string) {
  if (pickerMode.value === "rj") {
    rjLocalFolder.value = dir;
    // 提取父文件夹名作为智能分组
    const parts = dir.replace(/\\/g, "/").split("/").filter(Boolean);
    if (parts.length >= 2) {
      const parent = parts[parts.length - 2];
      if (!parent.toUpperCase().startsWith("RJ") && !parent.endsWith(":")) {
        rjGroupName.value = parent;
      }
    }
  } else {
    scanPickedDir(dir);
  }
}

async function doPreview() {
  const rj = rjInput.value.trim().toUpperCase();
  if (!/^RJ\d{4,}$/.test(rj)) {
    previewError.value = "RJ 号格式不对，例如 RJ01014447";
    return;
  }
  previewing.value = true;
  previewError.value = null;
  preview.value = null;
  importDone.value = null;
  importFeedback.value = null;
  try {
    preview.value = await api.previewByRj(rj, source.value);
  } catch (e) {
    previewError.value = String(e);
  } finally {
    previewing.value = false;
  }
}

async function doImport() {
  if (!preview.value) return;
  importing.value = true;
  importFeedback.value = null;
  try {
    const res = await api.importByRj(preview.value.rj_code, source.value);
    worksStore.upsertWork(res.work);
    importDone.value = res.work.work.id;

    // 若填选了本地文件夹，则自动关联并扫描音轨与匹配分组
    if (rjLocalFolder.value.trim()) {
      const gname = rjSmartGroup.value ? (rjGroupName.value.trim() || null) : null;
      const bindRes = await api.bindLocalFolder(preview.value.rj_code, rjLocalFolder.value.trim(), gname);
      worksStore.fetchWorks();
      worksStore.loadGroups();
      importFeedback.value = `已关联本地文件夹，扫描到 ${bindRes.trackCount} 首音轨${bindRes.groupName ? `，已分配分组「${bindRes.groupName}」` : ""}`;
    }
  } catch (e) {
    previewError.value = String(e);
  } finally {
    importing.value = false;
  }
}

// ============ 文件夹扫描 ============
const scanning = ref(false);
const scanItems = ref<ScannedItem[]>([]);
const scanDone = ref(false);
const importingAll = ref(false);
const smartGroup = ref(true); // 智能匹配分组（将上级分类文件夹设为分组）

async function scanPickedDir(dir: string) {
  scanning.value = true;
  scanItems.value = [];
  scanDone.value = false;
  try {
    const res = await api.scanFolder(dir);
    scanItems.value = res.items;
    scanDone.value = true;
  } catch (e) {
    previewError.value = String(e);
  } finally {
    scanning.value = false;
  }
}

async function importFound(item: ScannedItem) {
  try {
    await api.importByRj(item.rjCode, source.value);
    await api.bindLocalFolder(
      item.rjCode,
      item.path,
      smartGroup.value ? item.groupName : null
    );
    item.found = true;
    item.title = item.rjCode;
    worksStore.fetchWorks();
    worksStore.loadGroups();
  } catch (e) {
    alert(`导入 ${item.rjCode} 失败: ${e}`);
  }
}

/** 一键导入所有未入库的扫描结果 */
async function importAll() {
  const pending = scanItems.value.filter((i) => !i.found);
  if (!pending.length) {
    alert("没有需要导入的作品。");
    return;
  }
  if (!confirm(`将逐个导入 ${pending.length} 个作品（每个作品会联网抓取元数据），是否继续？`)) return;
  importingAll.value = true;
  let ok = 0;
  let fail = 0;
  try {
    for (const item of pending) {
      try {
        await api.importByRj(item.rjCode, source.value);
        await api.bindLocalFolder(
          item.rjCode,
          item.path,
          smartGroup.value ? item.groupName : null
        );
        item.found = true;
        item.title = item.rjCode;
        ok++;
      } catch (e) {
        fail++;
        console.warn(`导入 ${item.rjCode} 失败:`, e);
      }
    }
    worksStore.fetchWorks();
    worksStore.loadGroups();
  } finally {
    importingAll.value = false;
  }
  alert(`导入完成：成功 ${ok} 个，失败 ${fail} 个。`);
}
</script>

<template>
  <div class="p-5 space-y-6 max-w-4xl">
    <!-- ===== RJ 号导入 ===== -->
    <section class="card p-5">
      <h2 class="text-sm font-semibold text-white flex items-center gap-2 mb-1">
        <Plus :size="16" class="text-accent" /> 通过 RJ 号导入
      </h2>
      <p class="text-xs text-muted mb-4">
        输入作品的 RJ 号，自动抓取封面、标题、社团、声优、标签等信息。
      </p>

      <!-- 数据源切换 -->
      <div class="flex items-center gap-2 mb-3">
        <span class="text-[11px] text-muted shrink-0">数据源：</span>
        <div class="flex rounded-lg border border-bg-border overflow-hidden">
          <button
            v-for="s in sources"
            :key="s.value"
            class="px-3 py-1.5 text-xs transition-colors"
            :class="source === s.value ? 'bg-accent/20 text-accent-light' : 'text-muted hover:text-white'"
            @click="source = s.value as 'auto' | 'dlsite' | 'asmrone'"
          >
            {{ s.label }}
          </button>
        </div>
        <span class="text-[10px] text-muted/60">
          {{ sources.find((s) => s.value === source)?.desc }}
        </span>
      </div>

      <div class="flex gap-2">
        <input
          v-model="rjInput"
          class="input flex-1 font-mono uppercase"
          placeholder="例如: RJ01014447"
          @keyup.enter="doPreview"
        />
        <button class="btn-primary" :disabled="previewing" @click="doPreview">
          <Search v-if="!previewing" :size="15" />
          <Loader2 v-else :size="15" class="animate-spin" />
          预览
        </button>
      </div>

      <p v-if="previewError" class="text-xs text-accent mt-2">{{ previewError }}</p>

      <!-- 预览卡片 -->
      <div v-if="preview" class="mt-4 flex gap-4 card !bg-bg-deep p-4 animate-fade-in">
        <div class="w-28 aspect-[3/4] rounded-lg overflow-hidden bg-bg-hover shrink-0">
          <img v-if="preview.cover_url" :src="preview.cover_url" class="w-full h-full object-cover" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold text-white">{{ preview.title_zh || preview.title_ja }}</div>
          <div class="text-[11px] text-muted mt-0.5 font-mono">{{ preview.rj_code }}</div>
          <div class="flex flex-wrap gap-1.5 mt-2 text-[11px] text-muted">
            <span v-if="preview.circle_name">🏢 {{ preview.circle_name }}</span>
            <span v-if="preview.sale_date">📅 {{ preview.sale_date }}</span>
            <span v-if="preview.price">💰 ¥{{ preview.price.toLocaleString() }}</span>
            <span v-if="preview.duration_min">⏱ {{ preview.duration_min }}分</span>
            <span v-if="preview.file_size_mb">📦 {{ preview.file_size_mb.toFixed(1) }}MB</span>
          </div>
          <div v-if="preview.actors.length" class="mt-1.5 text-[11px] text-muted">
            🎤 {{ preview.actors.join(" / ") }}
          </div>
          <div v-if="preview.tags.length" class="mt-1.5 flex flex-wrap gap-1">
            <span
              v-for="t in preview.tags"
              :key="t"
              class="px-1.5 py-0.5 rounded text-[10px] bg-bg-hover text-muted"
            >
              #{{ t }}
            </span>
          </div>

          <!-- 关联本地文件夹选项 -->
          <div class="mt-3 pt-3 border-t border-bg-border/60 space-y-2">
            <div class="flex items-center gap-2">
              <span class="text-[11px] text-muted shrink-0">关联本地文件夹：</span>
              <input
                v-model="rjLocalFolder"
                class="input flex-1 !text-xs !py-1"
                placeholder="可选：选择该作品在硬盘中的存放目录"
              />
              <button class="btn-outline !text-xs !py-1 flex items-center gap-1" @click="pickRjLocalFolder">
                <FolderOpen :size="12" /> 浏览
              </button>
            </div>
            <div v-if="rjLocalFolder" class="flex items-center gap-4 text-[11px] text-muted">
              <label class="flex items-center gap-1.5 cursor-pointer select-none">
                <input type="checkbox" v-model="rjSmartGroup" class="rounded text-accent" />
                <span>智能匹配分组 (将上级目录设为分组)</span>
              </label>
              <div v-if="rjSmartGroup" class="flex items-center gap-1">
                <span>分组名:</span>
                <input v-model="rjGroupName" class="input !text-xs !py-0.5 !px-2 w-28" />
              </div>
            </div>
          </div>

          <p v-if="importFeedback" class="text-xs text-green-400 mt-2 font-medium">✅ {{ importFeedback }}</p>

          <div class="mt-3 flex gap-2">
            <button class="btn-primary !text-xs" :disabled="importing" @click="doImport">
              <Loader2 v-if="importing" :size="13" class="animate-spin" />
              <Plus v-else :size="13" />
              {{ importDone ? "已导入 ✓" : "确认导入" }}
            </button>
            <button
              v-if="importDone"
              class="btn-outline !text-xs"
              @click="router.push({ name: 'work-detail', params: { id: importDone } })"
            >
              查看详情
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ===== 本地文件夹扫描 ===== -->
    <section class="card p-5">
      <h2 class="text-sm font-semibold text-white flex items-center gap-2 mb-1">
        <FolderOpen :size="16" class="text-accent" /> 扫描本地文件夹
      </h2>
      <p class="text-xs text-muted mb-4">
        扫描硬盘上已下载的 DLsite 作品文件夹（自动识别 RJ 号），识别出已导入的作品。
      </p>

      <button class="btn-outline" :disabled="scanning" @click="pickerMode = 'scan'; showFolderPicker = true">
        <Loader2 v-if="scanning" :size="15" class="animate-spin" />
        <FolderOpen v-else :size="15" />
        {{ scanning ? "扫描中..." : "选择文件夹并扫描" }}
      </button>

      <div v-if="scanDone && !scanItems.length" class="mt-4 text-xs text-muted">
        未在所选文件夹中发现 RJ 号目录。
      </div>

      <div v-if="scanItems.length" class="mt-4 space-y-1.5 max-h-80 overflow-y-auto">
        <div class="flex flex-wrap items-center justify-between gap-2 px-1 pb-1.5">
          <span class="text-[11px] text-muted">
            共 {{ scanItems.length }} 个 RJ 目录，未导入 {{ scanItems.filter((i) => !i.found).length }} 个
          </span>
          <div class="flex items-center gap-3">
            <label class="flex items-center gap-1.5 text-[11px] text-muted cursor-pointer select-none">
              <input v-model="smartGroup" type="checkbox" class="accent-accent" />
              🔗 智能匹配分组（将上级分类文件夹设为分组）
            </label>
            <button class="btn-primary !text-[11px] !px-3 !py-1" :disabled="importingAll" @click="importAll">
              <Loader2 v-if="importingAll" :size="12" class="animate-spin" />
              <Download v-else :size="12" />
              {{ importingAll ? "导入中..." : "一键导入全部" }}
            </button>
          </div>
        </div>
        <div
          v-for="item in scanItems"
          :key="item.path"
          class="flex items-center gap-3 rounded-lg px-3 py-2 bg-bg-deep border border-bg-border"
        >
          <CheckCircle2 v-if="item.found" :size="15" class="text-green-400 shrink-0" />
          <XCircle v-else :size="15" class="text-muted shrink-0" />
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-xs text-white font-mono">{{ item.rjCode }}</span>
              <span
                v-if="item.groupName"
                class="px-1 py-px rounded text-[9px] bg-accent/10 text-accent-light shrink-0"
              >
                {{ item.groupName }}
              </span>
            </div>
            <div class="text-[10px] text-muted ellipsis-1">{{ item.path }}</div>
          </div>
          <span v-if="item.found" class="text-[10px] text-green-400 shrink-0">
            {{ item.title || "已入库" }}
          </span>
          <button
            v-else
            class="btn-primary !text-[10px] !px-2 !py-1 shrink-0"
            @click="importFound(item)"
          >
            <Download :size="11" /> 抓取导入
          </button>
        </div>
      </div>
    </section>

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      :title="pickerMode === 'rj' ? '选择要关联的本地作品文件夹（服务器上的目录）' : '选择要扫描的文件夹（服务器上的目录）'"
      @close="showFolderPicker = false"
      @select="handleFolderPicked"
    />
  </div>
</template>
