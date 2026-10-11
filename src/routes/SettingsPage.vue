<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import {
  Check,
  Loader2,
  HardDriveDownload,
  HardDriveUpload,
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

const THEMES: { name: ThemeName; label: string; sw: string[] }[] = [
  { name: "dark", label: "🌙 深色模式", sw: ["#0d0d0f", "#2f8fef", "#f5f5f7"] },
  { name: "light", label: "☀️ 浅色明亮", sw: ["#f5f5f7", "#0071e3", "#1c1c1e"] },
  { name: "sakura", label: "✨ 樱花粉系", sw: ["#fff0f3", "#ff6d92", "#5a2a38"] },
];

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
  <div class="page-wrap">
    <div class="page-head">
      <div>
        <h1>设置</h1>
        <p class="sub">主题、数据源、网络代理与 LLM 翻译配置</p>
      </div>
    </div>

    <!-- 主题 -->
    <section class="card set-card">
      <h2>🎨 主题风格</h2>
      <div class="theme-row">
        <button
          v-for="t in THEMES"
          :key="t.name"
          class="theme-card"
          :class="{ on: theme === t.name }"
          @click="selectTheme(t.name)"
        >
          <span class="sw">
            <i v-for="(c, i) in t.sw" :key="i" :style="{ background: c }" />
          </span>
          <b>{{ t.label }}</b>
        </button>
      </div>
    </section>

    <!-- 下载 -->
    <section class="card set-card">
      <h2>📦 下载设置</h2>
      <div class="set-row">
        <span class="k">默认下载目录</span>
        <div class="field"><input v-model="downloadDir" placeholder="默认下载目录" /></div>
        <button class="btn" @click="pickDir">浏览</button>
      </div>
    </section>

    <!-- asmr.one -->
    <section class="card set-card">
      <h2>🎧 asmr.one 数据源</h2>
      <p class="desc">
        用于在线浏览与下载音声。推荐直接填写下方<b style="color: var(--label)">站点地址、账号和密码</b>后「保存并登录」自动获取
        Token；也可手动获取：浏览器登录 asmr.one → F12 → 应用 → 本地存储 → 复制 <code>eyJ</code> 开头的 Token。
      </p>
      <div class="set-row">
        <span class="k">站点地址</span>
        <div class="field"><input v-model="asmrAddress" placeholder="asmr.one 地址(可选,默认 https://api.asmr.one,可填镜像)" /></div>
      </div>
      <div class="set-row">
        <span class="k">账号(邮箱)</span>
        <div class="field"><input v-model="asmrUsername" placeholder="账号(邮箱)" autocomplete="off" /></div>
      </div>
      <div class="set-row">
        <span class="k">密码</span>
        <div class="field"><input v-model="asmrPassword" type="password" placeholder="密码" autocomplete="new-password" @keyup.enter="saveAndLoginAsmr" /></div>
        <button class="btn primary" :disabled="asmrLoginLoading" @click="saveAndLoginAsmr">
          <Loader2 v-if="asmrLoginLoading" :size="13" class="animate-spin" />
          保存并登录
        </button>
      </div>
      <p v-if="asmrLoginMsg" class="desc" :style="{ color: asmrLoginMsg.ok ? 'var(--green)' : 'var(--red)' }">
        {{ asmrLoginMsg.ok ? "✅" : "⚠️" }} {{ asmrLoginMsg.text }}
      </p>
      <div class="set-row" style="align-items: flex-start">
        <span class="k" style="padding-top: 9px">Token</span>
        <div class="field auto">
          <textarea v-model="asmrToken" rows="2" class="mono" placeholder="粘贴 asmr.one 的 Token(eyJ... 开头)" />
        </div>
      </div>
      <div class="set-row">
        <span class="k" />
        <span class="note" :style="{ color: hasAsmrToken ? 'var(--green)' : 'var(--label-3)' }">
          {{ hasAsmrToken ? "✅ 已填 Token" : "未填写" }}
        </span>
        <button class="btn" :disabled="!hasAsmrToken || asmrSaved" @click="saveAsmrToken">
          <Check v-if="asmrSaved" :size="13" style="color: var(--green)" />
          保存 Token
        </button>
        <button class="btn danger" @click="clearAsmr">清除</button>
      </div>
    </section>

    <!-- 代理 -->
    <section class="card set-card">
      <h2>🌐 网络代理</h2>
      <div class="set-row">
        <span class="k">代理地址</span>
        <div class="field"><input v-model="proxyUrl" placeholder="代理地址,如 http://127.0.0.1:8502" @keyup.enter="saveProxy" /></div>
        <button class="btn" :disabled="proxySaving" @click="saveProxy">
          <Check v-if="proxySaved" :size="13" style="color: var(--green)" />
          保存
        </button>
      </div>
      <p class="desc">DLsite 与 asmr.one 的抓取、下载可分别决定是否走此代理（仅支持 HTTP 代理）。保存后对新发起的请求立即生效。</p>
      <div class="switch-row"><span class="lbl">DLsite 请求走代理</span><button type="button" role="switch" class="ios-switch" :class="{ on: proxyDlsite }" :aria-checked="proxyDlsite" @click="proxyDlsite = !proxyDlsite" /></div>
      <div class="switch-row"><span class="lbl">ASMR ONE 请求走代理</span><button type="button" role="switch" class="ios-switch" :class="{ on: proxyAsmrone }" :aria-checked="proxyAsmrone" @click="proxyAsmrone = !proxyAsmrone" /></div>
      <div class="switch-row"><span class="lbl">LLM 请求走代理</span><button type="button" role="switch" class="ios-switch" :class="{ on: proxyLlm }" :aria-checked="proxyLlm" @click="proxyLlm = !proxyLlm" /></div>
    </section>

    <!-- LLM -->
    <section class="card set-card">
      <h2>🈶 LLM 翻译</h2>
      <p class="desc">任意 OpenAI 兼容接口均可,例如:<span class="mono-inline">https://api.openai.com/v1</span></p>
      <div class="grid2">
        <div class="set-row tight"><span class="k">API 端点</span><div class="field"><input v-model="llmEndpoint" placeholder="https://api.deepseek.com/v1" /></div></div>
        <div class="set-row tight"><span class="k">API 密钥</span><div class="field"><input v-model="llmApiKey" type="password" placeholder="sk-..." autocomplete="new-password" /></div></div>
        <div class="set-row tight"><span class="k">模型</span><div class="field"><input v-model="llmModel" placeholder="deepseek-chat" /></div></div>
      </div>
      <div class="grid2">
        <div class="set-row tight"><span class="k">最大重试次数</span><div class="field"><input v-model.number="llmMaxRetry" type="number" min="0" max="10" /></div></div>
        <div class="set-row tight"><span class="k">每秒请求上限</span><div class="field"><input v-model.number="llmRps" type="number" min="1" max="100" /></div></div>
        <div class="set-row tight"><span class="k">翻译线程数(1-4)</span><div class="field"><input v-model.number="llmThreads" type="number" min="1" max="4" /></div></div>
      </div>
      <div class="switch-row"><span class="lbl">入库自动翻译(作品入库后自动加入翻译队列)</span><button type="button" role="switch" class="ios-switch" :class="{ on: llmAuto }" :aria-checked="llmAuto" @click="llmAuto = !llmAuto" /></div>
      <div class="set-row" style="align-items: flex-start">
        <span class="k" style="padding-top: 9px">翻译 Prompt</span>
        <div class="field auto"><textarea v-model="llmPrompt" rows="2" placeholder="翻译 Prompt(留空使用系统默认)" /></div>
      </div>
      <div class="save-bar" style="margin-top: 20px">
        <span class="hint">保存后对新发起的翻译任务生效</span>
        <button class="btn primary" :disabled="llmSaving" @click="saveLlm">
          <Check v-if="llmSaved" :size="13" style="color: #c9f2d4" />
          保存配置
        </button>
      </div>
    </section>

    <!-- 数据库 -->
    <section class="card set-card">
      <h2>🗄️ 数据库</h2>
      <div class="set-row">
        <span class="k">数据库文件</span>
        <span class="v">{{ dbPath || "加载中..." }}</span>
        <button class="btn" @click="api.openFolder(dbPath)">打开目录</button>
      </div>
      <p class="desc">数据迁移会将全部作品、标签、翻译与设置打包导出,或从备份文件还原。</p>
      <p class="amber-warn">⚠️ 导入还原将覆盖服务器上的当前所有数据!(当前数据库会自动备份为 .db.bak)</p>
      <div class="flex gap-2.5 mt-3.5">
        <button class="btn" @click="exportDb"><HardDriveDownload :size="14" /> 导出备份</button>
        <button class="btn danger" @click="triggerImportDb"><HardDriveUpload :size="14" /> 导入还原</button>
      </div>
    </section>

    <div class="save-bar">
      <span class="hint">部分设置保存后立即生效</span>
      <button class="btn primary" @click="save">
        <Loader2 v-if="saved" :size="14" class="animate-spin" />
        <Check v-else :size="14" style="color: #c9f2d4" />
        {{ saved ? "已保存" : "保存设置" }}
      </button>
    </div>

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      title="选择下载目录（服务器上的路径）"
      @close="showFolderPicker = false"
      @select="handleDirPicked"
    />

    <input
      ref="importFileInput"
      type="file"
      accept=".db,application/octet-stream"
      class="hidden"
      @change="onImportFile"
    />
  </div>
</template>

<style scoped>
.page-wrap {
  max-width: 980px;
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

.set-card {
  padding: 24px 26px;
}
.set-card + .set-card,
.set-card + .save-bar {
  margin-top: 18px;
}
.set-card > h2 {
  font-size: 16.5px;
  font-weight: 700;
  letter-spacing: -0.01em;
  display: flex;
  align-items: center;
  gap: 9px;
  color: var(--label);
}
.set-card > .desc {
  font-size: 12.5px;
  color: var(--label-3);
  margin-top: 6px;
  line-height: 1.7;
}
.set-row {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-top: 18px;
}
.set-row.tight {
  margin-top: 14px;
}
.set-row .k {
  width: 150px;
  font-size: 13px;
  font-weight: 600;
  color: var(--label-2);
  flex-shrink: 0;
}
.set-row.tight .k {
  width: 110px;
}
.set-row .v {
  flex: 1;
  font-size: 13px;
  color: var(--label-2);
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.set-row .note {
  flex: 1;
  font-size: 12.5px;
  color: var(--label-3);
}

.field {
  flex: 1;
  min-width: 0;
}
.field.auto {
  height: auto;
  padding: 10px 15px;
  background: var(--bg-secondary);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.08);
  display: flex;
  color: var(--label-3);
}
.field.auto:focus-within {
  box-shadow: 0 0 0 4px rgba(47, 143, 239, 0.35), var(--shadow-card);
}
textarea {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  resize: none;
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 11.5px;
  font-weight: 500;
  color: var(--label);
}
textarea::placeholder {
  color: var(--label-3);
}
.mono {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 11.5px;
}
.mono-inline {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
}

/* 主题卡（设计稿 .theme-row / .theme-card） */
.theme-row {
  display: flex;
  gap: 14px;
  margin-top: 18px;
}
.theme-card {
  flex: 1;
  border: none;
  border-radius: var(--r-lg);
  cursor: pointer;
  background: var(--fill-1);
  padding: 16px;
  text-align: left;
  font-family: inherit;
  transition: background 0.2s;
}
.theme-card:hover {
  background: var(--fill-2);
}
.theme-card.on {
  background: rgba(47, 143, 239, 0.15);
  box-shadow: inset 0 0 0 1.5px var(--blue);
}
.theme-card .sw {
  display: flex;
  gap: 6px;
}
.theme-card .sw i {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.18);
}
.theme-card b {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--label);
  margin-top: 11px;
}
.theme-card.on b {
  color: var(--blue-deep);
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
}
.switch-row .lbl {
  font-size: 13px;
  color: var(--label-2);
  font-weight: 500;
}

.save-bar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 14px;
  margin-top: 26px;
}
.save-bar .hint {
  font-size: 12px;
  color: var(--label-3);
}

.amber-warn {
  font-size: 11.5px;
  color: #ffb340;
  margin-top: 10px;
}
.grid2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0 16px;
}
@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
  .grid2 {
    grid-template-columns: 1fr;
  }
  .set-row {
    flex-wrap: wrap;
  }
  .set-row .k {
    width: 100%;
  }
}
</style>
