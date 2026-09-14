<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
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
  Globe,
  Languages,
} from "lucide-vue-next";
import * as api from "../lib/api";
import FolderPickerModal from "../components/common/FolderPickerModal.vue";
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

// 网络代理
const proxyUrl = ref("");
const proxyDlsite = ref(false);
const proxyAsmrone = ref(false);
const proxyLlm = ref(false);
const proxySaved = ref(false);
const proxySaving = ref(false);

// LLM 翻译
const llmEndpoint = ref("");
const llmApiKey = ref("");
const llmModel = ref("");
const llmMaxRetry = ref(3);
const llmRps = ref(2);
const llmThreads = ref(1);
const llmAuto = ref(false);
const llmPrompt = ref("");
const llmSaved = ref(false);
const llmSaving = ref(false);

// asmr.one 站点配置（地址 / 账号 / 密码，登录成功后 Token 自动保存）
const asmrAddress = ref("");
const asmrUsername = ref("");
const asmrPassword = ref("");
const asmrLoginLoading = ref(false);
const asmrLoginMsg = ref<{ ok: boolean; text: string } | null>(null);

// 目录选择器与数据库导入
const showFolderPicker = ref(false);
const importFileInput = ref<HTMLInputElement | null>(null);

onMounted(async () => {
  const s = await api.getSettings();
  const validTheme = (s.theme === "light" || s.theme === "sakura" ? s.theme : "dark") as ThemeName;
  theme.value = validTheme;
  applyTheme(validTheme);
  dbPath.value = await api.getDbPath();
  downloadStore.loadSettings();
  downloadDir.value = downloadStore.settings.download_dir;
  asmrToken.value = s.asmr_one_token || "";
  proxyUrl.value = s.proxy_url || "";
  proxyDlsite.value = s.proxy_dlsite;
  proxyAsmrone.value = s.proxy_asmrone;
  asmrAddress.value = s.asmr_one_address || "";
  asmrUsername.value = s.asmr_one_username || "";
  asmrPassword.value = s.asmr_one_password || "";
  proxyLlm.value = s.proxy_llm;
  llmEndpoint.value = s.llm_endpoint || "";
  llmApiKey.value = s.llm_api_key || "";
  llmModel.value = s.llm_model || "";
  llmMaxRetry.value = s.llm_max_retry;
  llmRps.value = s.llm_rps;
  llmThreads.value = s.llm_threads;
  llmAuto.value = s.llm_auto;
  llmPrompt.value = s.llm_prompt || "";
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

/** 保存网络代理配置（对所有新发起的请求立即生效） */
async function saveProxy() {
  proxySaving.value = true;
  try {
    await api.setSettings(undefined, undefined, {
      url: proxyUrl.value.trim(),
      dlsite: proxyDlsite.value,
      asmrone: proxyAsmrone.value,
      llm: proxyLlm.value,
    });
    proxySaved.value = true;
    setTimeout(() => (proxySaved.value = false), 1500);
  } catch (e) {
    alert(`保存失败：${e}`);
  } finally {
    proxySaving.value = false;
  }
}

/** 保存 LLM 翻译配置（对新发起的翻译任务生效） */
async function saveLlm() {
  llmSaving.value = true;
  try {
    await api.setSettings(undefined, undefined, undefined, undefined, {
      endpoint: llmEndpoint.value.trim(),
      apiKey: llmApiKey.value.trim(),
      model: llmModel.value.trim(),
      maxRetry: Number(llmMaxRetry.value) || 3,
      rps: Number(llmRps.value) || 2,
      threads: Number(llmThreads.value) || 1,
      auto: llmAuto.value,
      prompt: llmPrompt.value,
    });
    llmSaved.value = true;
    setTimeout(() => (llmSaved.value = false), 1500);
  } catch (e) {
    alert(`保存失败：${e}`);
  } finally {
    llmSaving.value = false;
  }
}

async function clearAsmr() {
  await api.clearAsmrToken();
  asmrToken.value = "";
}

/** 保存站点配置并用账号密码登录：成功后服务端自动保存新 Token */
async function saveAndLoginAsmr() {
  if (asmrLoginLoading.value) return;
  if (!asmrUsername.value.trim() || !asmrPassword.value) {
    asmrLoginMsg.value = { ok: false, text: "请先填写账号和密码" };
    return;
  }
  asmrLoginLoading.value = true;
  asmrLoginMsg.value = null;
  try {
    // 先持久化站点配置，再登录（登录端点会读取已保存的地址）
    await api.setSettings(undefined, undefined, undefined, {
      address: asmrAddress.value.trim(),
      username: asmrUsername.value.trim(),
      password: asmrPassword.value,
    });
    const res = await api.asmrLogin();
    asmrLoginMsg.value = { ok: true, text: res.message };
    // 拉取最新设置，刷新 Token 展示
    const s = await api.getSettings();
    asmrToken.value = s.asmr_one_token || "";
  } catch (e) {
    asmrLoginMsg.value = { ok: false, text: String(e) };
  } finally {
    asmrLoginLoading.value = false;
  }
}

/** 导出数据库备份：服务端生成快照，浏览器直接下载。 */
function exportDb() {
  api.exportDatabase();
}

function triggerImportDb() {
  importFileInput.value?.click();
}

async function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  if (!confirm("导入将覆盖服务器上的当前所有数据！\n（当前数据库会自动备份为 .db.bak）\n\n确定继续？")) return;
  try {
    const msg = await api.importDatabase(file);
    alert(msg);
    // 刷新界面数据
    dbPath.value = await api.getDbPath();
  } catch (e) {
    alert(`导入失败：${e}`);
  }
}

function pickDir() {
  showFolderPicker.value = true;
}

function handleDirPicked(dir: string) {
  downloadDir.value = dir;
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
        推荐直接填写下方的<b class="text-white/80">站点地址、账号和密码</b>，点击「保存并登录」即可自动获取
        Token（会在本机数据库中保存凭据）。也可以手动方式：用浏览器打开并登录
        <b class="text-white/80">www.asmr.one</b>，按 <b class="text-white/80">F12</b> →
        应用（Application）→ 本地存储（Local Storage）→ www.asmr.one，复制里面
        <b class="text-white/80">以 <code class="text-accent">eyJ</code> 开头</b>的长字符串（JWT），
        粘贴到下面保存即可。
      </p>
      <!-- 站点配置自动登录 -->
      <div class="space-y-2 mb-3">
        <input
          v-model="asmrAddress"
          class="input w-full !text-xs font-mono"
          placeholder="asmr.one 地址（可选，默认 https://api.asmr.one，可填镜像如 api.asmr-200.com）"
        />
        <div class="flex gap-2">
          <input
            v-model="asmrUsername"
            class="input flex-1 !text-xs"
            placeholder="账号（邮箱）"
            autocomplete="off"
          />
          <input
            v-model="asmrPassword"
            type="password"
            class="input flex-1 !text-xs"
            placeholder="密码"
            autocomplete="new-password"
            @keyup.enter="saveAndLoginAsmr"
          />
          <button class="btn-primary !text-xs shrink-0" :disabled="asmrLoginLoading" @click="saveAndLoginAsmr">
            <Loader2 v-if="asmrLoginLoading" :size="13" class="animate-spin" />
            <Check v-else :size="13" />
            保存并登录
          </button>
        </div>
        <p v-if="asmrLoginMsg" class="text-[11px]" :class="asmrLoginMsg.ok ? 'text-green-400' : 'text-accent'">
          {{ asmrLoginMsg.ok ? "✅" : "⚠️" }} {{ asmrLoginMsg.text }}
        </p>
      </div>
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

    <!-- 网络代理 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <Globe :size="14" class="text-accent" /> 网络代理
      </h3>
      <div class="flex gap-2">
        <input
          v-model="proxyUrl"
          class="input flex-1 !text-xs font-mono"
          placeholder="代理地址，如 http://127.0.0.1:8502"
          @keyup.enter="saveProxy"
        />
        <button class="btn-primary !text-xs" :disabled="proxySaving" @click="saveProxy">
          <Check v-if="!proxySaved" :size="13" />
          <Loader2 v-else :size="13" class="animate-spin" />
          {{ proxySaved ? "已保存" : "保存" }}
        </button>
      </div>
      <p class="text-[11px] text-muted mt-2">
        DLsite 与 asmr.one 的抓取、下载可分别决定是否走此代理（仅支持 HTTP 代理，地址可不带 http:// 前缀）。保存后对新发起的请求立即生效。
      </p>
      <div class="mt-3 pt-3 border-t border-bg-border space-y-2.5">
        <label class="flex items-center justify-between cursor-pointer select-none">
          <span class="text-xs text-white/80">DLsite 请求走代理</span>
          <button
            type="button"
            role="switch"
            :aria-checked="proxyDlsite"
            class="relative w-9 h-5 rounded-full transition-colors shrink-0"
            :class="proxyDlsite ? 'bg-accent' : 'bg-bg-hover border border-bg-border'"
            @click="proxyDlsite = !proxyDlsite"
          >
            <span
              class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
              :class="proxyDlsite ? 'translate-x-4' : ''"
            />
          </button>
        </label>
        <label class="flex items-center justify-between cursor-pointer select-none">
          <span class="text-xs text-white/80">ASMR ONE 请求走代理</span>
          <button
            type="button"
            role="switch"
            :aria-checked="proxyAsmrone"
            class="relative w-9 h-5 rounded-full transition-colors shrink-0"
            :class="proxyAsmrone ? 'bg-accent' : 'bg-bg-hover border border-bg-border'"
            @click="proxyAsmrone = !proxyAsmrone"
          >
            <span
              class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
              :class="proxyAsmrone ? 'translate-x-4' : ''"
            />
          </button>
        </label>
        <label class="flex items-center justify-between cursor-pointer select-none">
          <span class="text-xs text-white/80">LLM 请求走代理</span>
          <button
            type="button"
            role="switch"
            :aria-checked="proxyLlm"
            class="relative w-9 h-5 rounded-full transition-colors shrink-0"
            :class="proxyLlm ? 'bg-accent' : 'bg-bg-hover border border-bg-border'"
            @click="proxyLlm = !proxyLlm"
          >
            <span
              class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
              :class="proxyLlm ? 'translate-x-4' : ''"
            />
          </button>
        </label>
      </div>
    </section>

    <!-- LLM 翻译 -->
    <section class="card p-4">
      <h3 class="text-xs font-semibold text-white flex items-center gap-1.5 mb-3">
        <Languages :size="14" class="text-accent" /> LLM 翻译
      </h3>
      <p class="text-[11px] text-muted mb-3">
        使用 OpenAI 兼容接口把作品的日文标题/简介翻译成简体中文。API 端点填服务地址（如
        <code class="text-accent">https://api.deepseek.com/v1</code>，兼容 one-api、本地 Ollama 等）。
      </p>
      <div class="space-y-2">
        <input
          v-model="llmEndpoint"
          class="input w-full !text-xs font-mono"
          placeholder="API 端点，如 https://api.deepseek.com/v1"
        />
        <div class="flex gap-2">
          <input
            v-model="llmApiKey"
            type="password"
            class="input flex-1 !text-xs font-mono"
            placeholder="API 密钥"
            autocomplete="new-password"
          />
          <input
            v-model="llmModel"
            class="input flex-1 !text-xs font-mono"
            placeholder="模型，如 deepseek-chat"
          />
        </div>
        <div class="grid grid-cols-3 gap-2">
          <label class="text-[10px] text-muted">
            最大重试次数
            <input v-model.number="llmMaxRetry" type="number" min="0" max="10" class="input w-full !text-xs mt-0.5" />
          </label>
          <label class="text-[10px] text-muted">
            每秒请求上限
            <input v-model.number="llmRps" type="number" min="1" max="100" class="input w-full !text-xs mt-0.5" />
          </label>
          <label class="text-[10px] text-muted">
            翻译线程数（1-4）
            <input v-model.number="llmThreads" type="number" min="1" max="4" class="input w-full !text-xs mt-0.5" />
          </label>
        </div>
        <label class="flex items-center justify-between cursor-pointer select-none pt-1">
          <span class="text-xs text-white/80">入库自动翻译（作品入库后自动加入翻译队列）</span>
          <button
            type="button"
            role="switch"
            :aria-checked="llmAuto"
            class="relative w-9 h-5 rounded-full transition-colors shrink-0"
            :class="llmAuto ? 'bg-accent' : 'bg-bg-hover border border-bg-border'"
            @click="llmAuto = !llmAuto"
          >
            <span
              class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
              :class="llmAuto ? 'translate-x-4' : ''"
            />
          </button>
        </label>
        <label class="text-[10px] text-muted block">
          翻译 Prompt（留空使用系统默认）
          <textarea
            v-model="llmPrompt"
            rows="3"
            class="w-full rounded-lg bg-bg-hover border border-bg-border px-3 py-2 text-xs text-white font-mono resize-none outline-none focus:border-accent/60 mt-0.5"
            :placeholder="llmPrompt || '留空使用系统默认 Prompt'"
          ></textarea>
        </label>
      </div>
      <div class="flex items-center gap-3 mt-3">
        <span class="text-[11px] text-muted">保存后对新发起的翻译任务生效</span>
        <div class="flex-1" />
        <button class="btn-primary !text-xs" :disabled="llmSaving" @click="saveLlm">
          <Check v-if="!llmSaved" :size="13" />
          <Loader2 v-else :size="13" class="animate-spin" />
          {{ llmSaved ? "已保存" : "保存配置" }}
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
        <span>数据库存放在服务端程序同级目录下，备份只需复制该文件。也可使用下方导出 / 导入功能进行数据迁移。</span>
      </div>
      <!-- 数据迁移 -->
      <div class="flex items-center gap-2 mt-3 pt-3 border-t border-bg-border">
        <span class="text-xs text-white/70 mr-auto">数据迁移</span>
        <button class="btn-outline !text-xs flex items-center gap-1" @click="exportDb">
          <HardDriveDownload :size="13" /> 导出备份
        </button>
        <button class="btn-outline !text-xs flex items-center gap-1 !border-amber-500/40 !text-amber-400 hover:!bg-amber-500/10" @click="triggerImportDb">
          <HardDriveUpload :size="13" /> 导入还原
        </button>
        <input
          ref="importFileInput"
          type="file"
          accept=".db,application/octet-stream"
          class="hidden"
          @change="onImportFile"
        />
      </div>
    </section>

    <!-- 保存 -->
    <button class="btn-primary" @click="save">
      <Loader2 v-if="saved" :size="15" class="animate-spin" />
      <Check v-else :size="15" />
      {{ saved ? "已保存" : "保存设置" }}
    </button>

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      title="选择下载目录（服务器上的路径）"
      @close="showFolderPicker = false"
      @select="handleDirPicked"
    />
  </div>
</template>
