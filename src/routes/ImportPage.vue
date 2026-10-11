<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import {
  FolderOpen,
  Loader2,
  Search,
  Check,
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
  { value: "auto", label: "智能自动", desc: "优先 asmr.one,失败回退 DLsite" },
  { value: "dlsite", label: "DLsite", desc: "从 DLsite 抓取" },
  { value: "asmrone", label: "asmr.one", desc: "从 ASMR ONE 抓取" },
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
      importFeedback.value = `已关联本地文件夹,扫描到 ${bindRes.trackCount} 首音轨${bindRes.groupName ? `,已分配分组「${bindRes.groupName}」` : ""}`;
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
  <div class="page-wrap">
    <div class="page-head">
      <div>
        <h1>导入作品</h1>
        <p class="sub">通过 RJ 号从 DLsite / asmr.one 拉取元数据,或扫描本地文件夹批量入库</p>
      </div>
    </div>

    <div class="cols">
      <!-- 左:RJ 号导入 -->
      <section class="card panel">
        <h2>📥 通过 RJ 号导入</h2>
        <p class="desc">输入作品的 RJ 号,先预览元数据再确认入库</p>

        <div class="src-seg">
          <button
            v-for="s in sources"
            :key="s.value"
            class="src-opt"
            :class="{ on: source === s.value }"
            @click="source = s.value as 'auto' | 'dlsite' | 'asmrone'"
          >
            <b>{{ s.label }}</b>
            <span>{{ s.desc }}</span>
          </button>
        </div>

        <div class="rj-input-row">
          <label class="field">
            <Search :size="14" />
            <input
              v-model="rjInput"
              class="mono"
              placeholder="例如: RJ01014447"
              @keyup.enter="doPreview"
            />
          </label>
          <button class="btn primary" style="height: 44px" :disabled="previewing" @click="doPreview">
            <Loader2 v-if="previewing" :size="15" class="animate-spin" />
            <Search v-else :size="15" />
            预览
          </button>
        </div>

        <p v-if="previewError" class="text-xs mt-2.5" style="color: var(--red)">{{ previewError }}</p>

        <!-- 预览卡片 -->
        <div v-if="preview" class="card preview animate-fade-in">
          <div class="pv-cov">
            <img v-if="preview.cover_url" :src="preview.cover_url" />
          </div>
          <div class="min-w-0">
            <h3>{{ preview.title_zh || preview.title_ja }}</h3>
            <p class="pv-rj">{{ preview.rj_code }}</p>
            <div class="pv-meta">
              <span v-if="preview.circle_name">🏢 {{ preview.circle_name }}</span>
              <span v-if="preview.sale_date">📅 {{ preview.sale_date }}</span>
              <span v-if="preview.price">💰 {{ preview.price.toLocaleString() }} 円</span>
              <span v-if="preview.actors.length">🎤 {{ preview.actors.join(" / ") }}</span>
              <span v-if="preview.file_size_mb">📦 {{ preview.file_size_mb.toFixed(1) }} MB</span>
            </div>
            <div v-if="preview.tags.length" class="pv-tags">
              <span v-for="t in preview.tags" :key="t" class="btag">{{ t }}</span>
            </div>
          </div>
        </div>

        <div v-if="preview" class="assoc">
          <h4>关联本地文件夹(可选)</h4>
          <div class="assoc-row">
            <label class="field">
              <FolderOpen :size="14" />
              <input v-model="rjLocalFolder" placeholder="选择本地作品文件夹" />
            </label>
            <button class="btn" style="height: 40px" @click="pickRjLocalFolder">浏览</button>
          </div>
          <label class="checkline">
            <button type="button" class="cbx" :class="{ off: !rjSmartGroup }" @click="rjSmartGroup = !rjSmartGroup">
              <Check v-if="rjSmartGroup" :size="10" stroke-width="3" />
            </button>
            智能匹配分组(将上级目录设为分组)
          </label>
          <div v-if="rjSmartGroup && rjLocalFolder" class="flex items-center gap-2 mt-2">
            <span class="text-[11px]" style="color: var(--label-3)">分组名:</span>
            <input v-model="rjGroupName" class="input !h-8 !py-0 !px-2.5 w-28 !text-xs" />
          </div>
          <p v-if="importFeedback" class="ok-note">✅ {{ importFeedback }}</p>
        </div>

        <div v-if="preview" class="import-actions">
          <button class="btn primary" :disabled="importing" @click="doImport">
            <Loader2 v-if="importing" :size="14" class="animate-spin" />
            {{ importDone ? "已导入 ✓" : "确认导入" }}
          </button>
          <button
            v-if="importDone"
            class="btn"
            @click="router.push({ name: 'work-detail', params: { id: importDone } })"
          >
            查看详情
          </button>
        </div>
      </section>

      <!-- 右:扫描本地文件夹 -->
      <section class="card panel">
        <h2>📂 扫描本地文件夹</h2>
        <p class="desc">选择包含 RJ 命名目录的文件夹,自动发现可导入的作品</p>

        <button class="btn primary" style="margin-top: 16px; height: 42px" :disabled="scanning" @click="pickerMode = 'scan'; showFolderPicker = true">
          <Loader2 v-if="scanning" :size="15" class="animate-spin" />
          {{ scanning ? "扫描中..." : "选择文件夹并扫描" }}
        </button>

        <div v-if="scanDone && !scanItems.length" class="desc mt-4">
          未在所选文件夹中发现 RJ 号目录。
        </div>

        <template v-if="scanItems.length">
          <div class="scan-head">
            <span class="info">
              共 <b>{{ scanItems.length }}</b> 个 RJ 目录,未导入 <b>{{ scanItems.filter((i) => !i.found).length }}</b> 个
            </span>
            <label class="checkline" style="margin-top: 0">
              <button type="button" class="cbx" :class="{ off: !smartGroup }" @click="smartGroup = !smartGroup">
                <Check v-if="smartGroup" :size="10" stroke-width="3" />
              </button>
              🔗 智能匹配分组
            </label>
          </div>

          <div class="scan-list">
            <div v-for="item in scanItems" :key="item.path" class="scan-item" :class="{ done: item.found }">
              <span class="st">
                <CheckCircle2 v-if="item.found" :size="15" style="color: var(--green)" />
                <XCircle v-else :size="15" style="color: var(--label-3)" />
              </span>
              <span class="rj">{{ item.rjCode }}</span>
              <span v-if="item.groupName" class="grp">{{ item.groupName }}</span>
              <span class="path" :title="item.path">{{ item.path }}</span>
              <span v-if="item.found" class="title-ok">已入库<template v-if="item.title"> · {{ item.title }}</template></span>
              <button
                v-else
                class="btn primary sm shrink-0"
                @click="importFound(item)"
              >
                <Download :size="11" /> 抓取导入
              </button>
            </div>
          </div>

          <div style="margin-top: 18px">
            <button class="btn primary" style="height: 42px" :disabled="importingAll" @click="importAll">
              <Loader2 v-if="importingAll" :size="14" class="animate-spin" />
              一键导入全部({{ scanItems.filter((i) => !i.found).length }})
            </button>
          </div>
        </template>
      </section>
    </div>

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      :title="pickerMode === 'rj' ? '选择要关联的本地作品文件夹（服务器上的目录）' : '选择要扫描的文件夹（服务器上的目录）'"
      @close="showFolderPicker = false"
      @select="handleFolderPicked"
    />
  </div>
</template>

<style scoped>
.page-wrap {
  max-width: 1280px;
  margin: 0 auto;
  padding: 26px 28px 150px;
}
.page-head {
  margin-bottom: 22px;
}
.page-head h1 {
  font-size: 26px;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.page-head .sub {
  font-size: 13px;
  color: var(--label-2);
  margin-top: 3px;
}

.cols {
  display: grid;
  grid-template-columns: 1fr 1.25fr;
  gap: 22px;
  align-items: start;
}
@media (max-width: 980px) {
  .cols {
    grid-template-columns: 1fr;
  }
}
.panel {
  padding: 24px 26px;
}
.panel > h2 {
  font-size: 16.5px;
  font-weight: 700;
  letter-spacing: -0.01em;
  display: flex;
  align-items: center;
  gap: 9px;
  color: var(--label);
}
.panel > .desc {
  font-size: 12.5px;
  color: var(--label-3);
  margin-top: 5px;
}

/* 数据源三选卡（设计稿 .src-seg / .src-opt） */
.src-seg {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-top: 18px;
}
.src-opt {
  border: none;
  border-radius: var(--r-md);
  background: var(--fill-1);
  padding: 12px 14px;
  text-align: left;
  cursor: pointer;
  transition: background 0.2s;
  font-family: inherit;
}
.src-opt:hover {
  background: var(--fill-2);
}
.src-opt.on {
  background: rgba(47, 143, 239, 0.18);
  box-shadow: inset 0 0 0 1.5px var(--blue);
}
.src-opt b {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--label);
}
.src-opt.on b {
  color: var(--blue-deep);
}
.src-opt span {
  display: block;
  font-size: 11px;
  color: var(--label-3);
  margin-top: 3px;
}

.rj-input-row {
  display: flex;
  gap: 10px;
  margin-top: 18px;
}
.rj-input-row .field {
  flex: 1;
  min-width: 0;
}
.mono {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  letter-spacing: 0.04em;
}

/* 预览卡（设计稿 .preview） */
.preview {
  margin-top: 20px;
  padding: 16px;
  display: flex;
  gap: 16px;
  box-shadow: none;
  background: var(--fill-1);
}
.pv-cov {
  width: 108px;
  aspect-ratio: 3 / 4;
  border-radius: 12px;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: var(--shadow-card);
  background: var(--fill-2);
}
.pv-cov img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.preview h3 {
  font-size: 14.5px;
  font-weight: 600;
  line-height: 1.45;
  color: var(--label);
}
.pv-rj {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 11.5px;
  color: var(--label-3);
  margin-top: 4px;
}
.pv-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 5px 16px;
  margin-top: 9px;
  font-size: 12px;
  color: var(--label-2);
}
.pv-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}
.btag {
  font-size: 11px;
  font-weight: 500;
  color: var(--label-2);
  background: var(--fill-2);
  border-radius: 999px;
  padding: 3px 10px;
}

/* 关联文件夹 */
.assoc {
  margin-top: 20px;
}
.assoc h4 {
  font-size: 13px;
  font-weight: 600;
  color: var(--label-2);
}
.assoc-row {
  display: flex;
  gap: 10px;
  margin-top: 10px;
}
.assoc-row .field {
  flex: 1;
  min-width: 0;
  height: 40px;
}
.checkline {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-top: 12px;
  font-size: 12.5px;
  color: var(--label-2);
  cursor: pointer;
  user-select: none;
}
.cbx {
  width: 17px;
  height: 17px;
  border-radius: 5.5px;
  background: var(--blue);
  border: none;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  color: #fff;
  cursor: pointer;
  padding: 0;
}
.cbx.off {
  background: var(--fill-2);
  color: transparent;
}
.import-actions {
  display: flex;
  gap: 10px;
  margin-top: 20px;
}
.ok-note {
  font-size: 12px;
  color: #4cd964;
  margin-top: 12px;
}

/* 扫描列表（设计稿 .scan-*） */
.scan-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 18px;
  flex-wrap: wrap;
}
.scan-head .info {
  font-size: 12.5px;
  color: var(--label-2);
}
.scan-head .info b {
  color: var(--label);
}
.scan-list {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.scan-item {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--fill-1);
  border-radius: var(--r-md);
  padding: 11px 14px;
  font-size: 12.5px;
}
.scan-item.done {
  opacity: 0.75;
}
.scan-item .st {
  width: 20px;
  text-align: center;
  flex-shrink: 0;
  display: grid;
  place-items: center;
}
.scan-item .rj {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-weight: 600;
  flex-shrink: 0;
}
.scan-item .grp {
  font-size: 10.5px;
  font-weight: 600;
  color: #6cb1f7;
  background: rgba(47, 143, 239, 0.13);
  border-radius: 6px;
  padding: 2px 8px;
  flex-shrink: 0;
}
.scan-item .path {
  flex: 1;
  min-width: 0;
  color: var(--label-3);
  font-size: 11.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
}
.scan-item .title-ok {
  color: #4cd964;
  font-size: 12px;
  flex-shrink: 0;
  max-width: 40%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}
</style>
