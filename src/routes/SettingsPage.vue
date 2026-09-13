<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  Moon,
  Sun,
  Sparkles,
  Palette,
  Database,
  FolderOpen,
  Info,
  Check,
  Loader2,
  Monitor,
  Trash2,
  HardDriveDownload,
  HardDriveUpload,
} from "lucide-vue-next";
import * as api from "../lib/api";
import { useDownloadStore } from "../stores/download";
import { applyTheme, type ThemeName } from "../lib/theme";

const downloadStore = useDownloadStore();

const theme = ref<ThemeName>("dark");
const dbPath = ref("");
const saved = ref(false);

const downloadDir = ref("");
const asmrToken = ref("");
const asmrSaved = ref(false);

const hasAsmrToken = computed(() => asmrToken.value.trim().length > 0);

onMounted(async () => {
  const s = await api.getSettings();
  const validTheme = (s.theme === "light" || s.theme === "sakura" ? s.theme : "dark") as ThemeName;
  theme.value = validTheme;
  applyTheme(validTheme);
  dbPath.value = await api.getDbPath();
  downloadStore.loadSettings();
  downloadDir.value = downloadStore.settings.download_dir;
  asmrToken.value = s.asmr_one_token || "";
});

function selectTheme(t: ThemeName) {
  theme.value = t;
  applyTheme(t);
}

async function save() {
  await api.setSettings(theme.value, downloadDir.value || undefined);
  applyTheme(theme.value);
  saved.value = true;
  setTimeout(() => (saved.value = false), 1500);
}

async function saveAsmrToken() {
  const t = asmrToken.value.trim();
  if (!t) return;
  try {
    await api.setAsmrToken(t);
    asmrSaved.value = true;
    setTimeout(() => (asmrSaved.value = false), 1500);
  } catch (e) {
    console.error("保存 asmr.one Token 失败", e);
    alert(`保存失败：${e}`);
  }
}

async function clearAsmr() {
  await api.clearAsmrToken();
  asmrToken.value = "";
}

async function exportDb() {
  const dest = await saveDialog({
    title: "导出数据库备份",
    defaultPath: "dlsite_manager_backup.db",
    filters: [{ name: "SQLite 数据库", extensions: ["db"] }],
  });
  if (!dest) return;
  try {
    const msg = await api.exportDatabase(dest);
    alert(msg);
  } catch (e) {
    alert(`导出失败：${e}`);
  }
}

async function importDb() {
  const src = await open({
    title: "选择要导入的数据库文件",
    multiple: false,
    filters: [{ name: "SQLite 数据库", extensions: ["db"] }],
  });
  if (!src || typeof src !== "string") return;
  if (!confirm("导入将覆盖当前所有数据！\n（当前数据库会自动备份为 .db.bak）\n\n确定继续？")) return;
  try {
    const msg = await api.importDatabase(src);
    alert(msg);
    // 刷新界面数据
    dbPath.value = await api.getDbPath();
  } catch (e) {
    alert(`导入失败：${e}`);
  }
}

async function pickDir() {
  const dir = await open({ directory: true, multiple: false, title: "选择下载目录" });
  if (dir && typeof dir === "string") {
    downloadDir.value = dir;
  }
}
</script>

<template>
  <div class="p-5 max-w-2xl space-y-5">
    <!-- 主题 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <Palette :size="14" class="text-accent" /> 主题风格
      </h3>
      <div class="grid grid-cols-3 gap-3">
        <!-- 深色 -->
        <button
          class="flex flex-col items-center justify-center gap-2 p-3 rounded-xl border text-sm transition-all"
          :class="theme === 'dark' ? 'border-accent bg-accent/15 text-accent-light shadow-md shadow-accent/10 ring-1 ring-accent/30' : 'border-bg-border text-muted hover:text-white hover:bg-bg-hover'"
          @click="selectTheme('dark')"
        >
          <div class="flex items-center gap-1.5">
            <Moon :size="16" />
            <span class="font-medium text-xs">深色模式</span>
          </div>
          <div class="flex gap-1">
            <span class="w-3 h-3 rounded-full bg-[#12121e] border border-white/20" />
            <span class="w-3 h-3 rounded-full bg-[#1a1a2e]" />
            <span class="w-3 h-3 rounded-full bg-[#e94560]" />
          </div>
        </button>

        <!-- 浅色 -->
        <button
          class="flex flex-col items-center justify-center gap-2 p-3 rounded-xl border text-sm transition-all"
          :class="theme === 'light' ? 'border-accent bg-accent/15 text-accent-light shadow-md shadow-accent/10 ring-1 ring-accent/30' : 'border-bg-border text-muted hover:text-white hover:bg-bg-hover'"
          @click="selectTheme('light')"
        >
          <div class="flex items-center gap-1.5">
            <Sun :size="16" />
            <span class="font-medium text-xs">浅色明亮</span>
          </div>
          <div class="flex gap-1">
            <span class="w-3 h-3 rounded-full bg-[#f6f8fc] border border-black/10" />
            <span class="w-3 h-3 rounded-full bg-[#ffffff] border border-black/10" />
            <span class="w-3 h-3 rounded-full bg-[#e11d48]" />
          </div>
        </button>

        <!-- 粉色系 -->
        <button
          class="flex flex-col items-center justify-center gap-2 p-3 rounded-xl border text-sm transition-all"
          :class="theme === 'sakura' ? 'border-accent bg-accent/15 text-accent-light shadow-md shadow-accent/10 ring-1 ring-accent/30' : 'border-bg-border text-muted hover:text-white hover:bg-bg-hover'"
          @click="selectTheme('sakura')"
        >
          <div class="flex items-center gap-1.5">
            <Sparkles :size="16" class="text-pink-400" />
            <span class="font-medium text-xs">樱花粉系</span>
          </div>
          <div class="flex gap-1">
            <span class="w-3 h-3 rounded-full bg-[#fff5f8] border border-pink-300" />
            <span class="w-3 h-3 rounded-full bg-[#fce7f3] border border-pink-300" />
            <span class="w-3 h-3 rounded-full bg-[#ec4899]" />
          </div>
        </button>
      </div>
    </section>

    <!-- 下载 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <FolderOpen :size="14" class="text-accent" /> 下载设置
      </h3>
      <div class="flex gap-2">
        <input
          v-model="downloadDir"
          class="input flex-1 !text-xs"
          placeholder="默认下载目录"
        />
        <button class="btn-outline" @click="pickDir">浏览</button>
      </div>
    </section>

    <!-- asmr.one 数据源 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <Monitor :size="14" class="text-accent" /> asmr.one 数据源
      </h3>
      <p class="text-[11px] text-muted mb-2">
        asmr.one 有反爬，程序直接请求会被拦截。用浏览器打开并登录
        <b class="text-white/80">www.asmr.one</b>，按 <b class="text-white/80">F12</b> →
        应用（Application）→ 本地存储（Local Storage）→ www.asmr.one，复制里面
        <b class="text-white/80">以 <code class="text-accent">eyJ</code> 开头</b>的长字符串（JWT），
        粘贴到下面保存即可。
      </p>
      <textarea
        v-model="asmrToken"
        rows="3"
        class="w-full rounded-lg bg-bg-hover border border-bg-border px-3 py-2 text-xs text-white font-mono resize-none outline-none focus:border-accent/60"
        placeholder="粘贴 asmr.one 的 Token（eyJ... 开头）"
      ></textarea>
      <div class="flex items-center gap-3 mt-3">
        <span class="text-xs" :class="hasAsmrToken ? 'text-green-400' : 'text-muted'">
          {{ hasAsmrToken ? "✅ 已填 Token" : "⚪ 未填写" }}
        </span>
        <div class="flex-1" />
        <button class="btn-primary !text-xs" :disabled="!hasAsmrToken" @click="saveAsmrToken">
          <Check v-if="!asmrSaved" :size="13" />
          <Loader2 v-else :size="13" class="animate-spin" />
          {{ asmrSaved ? "已保存" : "保存 Token" }}
        </button>
        <button v-if="hasAsmrToken" class="btn-ghost !text-xs !text-accent" @click="clearAsmr">
          <Trash2 :size="13" /> 清除
        </button>
      </div>
    </section>

    <!-- 数据库 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <Database :size="14" class="text-accent" /> 数据库
      </h3>
      <div class="flex items-center gap-2">
        <span class="text-xs text-muted ellipsis-1 flex-1 font-mono">{{ dbPath || "加载中..." }}</span>
        <button class="btn-outline !text-xs" @click="api.openFolder(dbPath)">打开目录</button>
      </div>
      <div class="text-[11px] text-muted mt-2 flex items-start gap-1.5">
        <Info :size="12" class="mt-0.5 shrink-0" />
        <span>数据库存放在软件同级目录下，备份只需复制该文件。也可使用下方导出 / 导入功能进行数据迁移。</span>
      </div>
      <!-- 数据迁移 -->
      <div class="flex items-center gap-2 mt-3 pt-3 border-t border-bg-border">
        <span class="text-xs text-white/70 mr-auto">数据迁移</span>
        <button class="btn-outline !text-xs flex items-center gap-1" @click="exportDb">
          <HardDriveDownload :size="13" /> 导出备份
        </button>
        <button class="btn-outline !text-xs flex items-center gap-1 !border-amber-500/40 !text-amber-400 hover:!bg-amber-500/10" @click="importDb">
          <HardDriveUpload :size="13" /> 导入还原
        </button>
      </div>
    </section>

    <!-- 保存 -->
    <button class="btn-primary" @click="save">
      <Loader2 v-if="saved" :size="15" class="animate-spin" />
      <Check v-else :size="15" />
      {{ saved ? "已保存" : "保存设置" }}
    </button>
  </div>
</template>
