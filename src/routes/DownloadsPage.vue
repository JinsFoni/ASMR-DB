<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import {
  Download,
  Pause,
  Play,
  X,
  Loader2,
  Check,
  TriangleAlert,
} from "lucide-vue-next";
import { useDownloadStore } from "../stores/download";
import * as api from "../lib/api";
import FolderPickerModal from "../components/common/FolderPickerModal.vue";
import EmptyState from "../components/common/EmptyState.vue";

const store = useDownloadStore();

const editingDir = ref(false);
const dirInput = ref("");
const showFolderPicker = ref(false);

const fmtBytes = (b: number) => {
  if (!b) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = b;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
};

const fmtNum = (b: number) => {
  if (!b) return "0";
  return b >= 1024 ** 3 ? (b / 1024 ** 3).toFixed(1) + " GB" : (b / 1024 ** 2).toFixed(1) + " MB";
};

/** 设计稿 .ticon / .st 状态 */
const statusMeta = computed(() => (s: string) => {
  switch (s) {
    case "downloading":
      return { label: "下载中", ticon: "run", st: "run", icon: Download };
    case "done":
      return { label: "已完成", ticon: "done", st: "done", icon: Check };
    case "paused":
      return { label: "已暂停", ticon: "wait", st: "wait", icon: Pause };
    case "error":
      return { label: "失败", ticon: "fail", st: "fail", icon: TriangleAlert };
    case "cancelled":
      return { label: "已取消", ticon: "fail", st: "fail", icon: X };
    default:
      return { label: "等待中", ticon: "wait", st: "wait", icon: Loader2 };
  }
});

onMounted(async () => {
  store.loadSettings();
  await store.refresh();
});

function pickDir() {
  showFolderPicker.value = true;
}

async function saveSettings() {
  if (!dirInput.value) return;
  await store.saveSettings(dirInput.value, store.settings.concurrency);
  editingDir.value = false;
}

async function doPause(rj: string) {
  await api.pauseDownload(rj);
  await store.refresh();
}
async function doResume(rj: string) {
  await api.resumeDownload(rj);
  await store.refresh();
}
async function doCancel(rj: string) {
  await api.cancelDownload(rj);
  await store.refresh();
}
</script>

<template>
  <div class="page-wrap">
    <!-- 页头 -->
    <div class="page-head">
      <div>
        <h1>下载管理</h1>
        <p class="sub">从 asmr.one 下载作品音声到本地目录</p>
      </div>
    </div>

    <!-- 下载设置 -->
    <section class="card dl-set">
      <div class="head">
        <h2>⚙️ 下载设置</h2>
        <button v-if="!editingDir" class="btn sm" @click="editingDir = true; dirInput = store.settings.download_dir">
          修改
        </button>
      </div>
      <div class="dl-row">
        <span class="k">下载目录</span>
        <template v-if="!editingDir">
          <span class="path">{{ store.settings.download_dir || "未设置" }}</span>
        </template>
        <template v-else>
          <input v-model="dirInput" class="input !h-10 flex-1 !text-xs min-w-[220px]" placeholder="选择下载目录" />
          <button class="btn sm" @click="pickDir">浏览</button>
          <button class="btn primary sm" @click="saveSettings">保存</button>
        </template>
      </div>
      <div class="dl-row">
        <span class="k">并发数</span>
        <select v-model.number="store.settings.concurrency" class="seg-select" @change="store.saveSettings(store.settings.download_dir, store.settings.concurrency)">
          <option :value="1">1</option>
          <option :value="2">2</option>
          <option :value="3">3</option>
          <option :value="4">4</option>
        </select>
      </div>
    </section>

    <!-- 下载队列 -->
    <div class="q-head">
      <h2>下载队列 <span class="n">{{ store.tasks.length }}</span></h2>
    </div>

    <div v-if="!store.tasks.length" class="card">
      <EmptyState
        title="没有下载任务"
        desc="在作品详情页点击「下载」并填入下载链接，即可开始下载。"
        :icon="Download"
      />
    </div>

    <template v-else>
      <section v-for="t in store.tasks" :key="t.rj_code" class="card task">
        <div class="ticon" :class="statusMeta(t.status).ticon">
          <Loader2 v-if="t.status === 'downloading'" :size="15" class="animate-spin" />
          <component :is="statusMeta(t.status).icon" v-else :size="15" />
        </div>
        <div class="tinfo">
          <div class="l1">
            <span class="rj">{{ t.rj_code }}</span>
            <span class="st" :class="statusMeta(t.status).st">{{ statusMeta(t.status).label }}</span>
            <span class="spd">
              <template v-if="t.status === 'downloading' && t.speed > 0">{{ t.speed.toFixed(1) }} MB/s</template>
              <template v-else-if="t.status === 'done'">{{ fmtBytes(t.bytes_total) }}</template>
              <template v-else>—</template>
            </span>
          </div>
          <div class="l2">
            <div class="prog" :class="{ green: t.status === 'done' }">
              <i :style="{ width: Math.min(100, t.progress) + '%' }" />
            </div>
            <span class="bytes">{{ fmtNum(t.bytes_done) }} / {{ fmtNum(t.bytes_total) }}</span>
          </div>
          <p v-if="t.error" class="errmsg">⚠️ {{ t.error }}</p>
        </div>
        <div class="ops">
          <button
            v-if="t.status === 'downloading'"
            class="opbtn" title="暂停"
            @click="doPause(t.rj_code)"
          >
            <Pause :size="12" />
          </button>
          <button
            v-if="t.status === 'paused' || t.status === 'error'"
            class="opbtn" title="继续"
            @click="doResume(t.rj_code)"
          >
            <Play :size="12" />
          </button>
          <button
            v-if="['downloading', 'paused', 'queued', 'error'].includes(t.status)"
            class="opbtn x" title="取消"
            @click="doCancel(t.rj_code)"
          >
            <X :size="11" />
          </button>
        </div>
      </section>
    </template>

    <!-- 服务端目录选择器 Modal -->
    <FolderPickerModal
      :visible="showFolderPicker"
      title="选择下载目录（服务器上的路径）"
      @close="showFolderPicker = false"
      @select="(dir) => (dirInput = dir)"
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
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

/* 下载设置卡 */
.dl-set {
  padding: 22px 26px;
}
.dl-set .head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.dl-set h2 {
  font-size: 16.5px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 9px;
  color: var(--label);
}
.dl-row {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-top: 16px;
}
.dl-row .k {
  width: 70px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--label-3);
  flex-shrink: 0;
}
.dl-row .path {
  flex: 1;
  font-size: 13px;
  color: var(--label-2);
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 队列标题 */
.q-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 30px 0 14px;
}
.q-head h2 {
  font-size: 17px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--label);
}
.q-head h2 .n {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--label-3);
  background: var(--fill-1);
  border-radius: 999px;
  padding: 2px 10px;
}

/* 任务卡（设计稿 .task） */
.task {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 15px 18px;
}
.task + .task {
  margin-top: 10px;
}
.ticon {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
}
.ticon.run {
  background: rgba(47, 143, 239, 0.15);
  color: #6cb1f7;
}
.ticon.done {
  background: rgba(48, 209, 88, 0.15);
  color: #4cd964;
}
.ticon.wait {
  background: var(--fill-2);
  color: var(--label-3);
}
.ticon.fail {
  background: rgba(255, 69, 58, 0.15);
  color: #ff6961;
}
.tinfo {
  flex: 1;
  min-width: 0;
}
.l1 {
  display: flex;
  align-items: center;
  gap: 10px;
}
.rj {
  font-family: "SF Mono", ui-monospace, Menlo, monospace;
  font-size: 12.5px;
  font-weight: 600;
}
.st {
  font-size: 11.5px;
  font-weight: 600;
}
.st.run {
  color: #6cb1f7;
}
.st.done {
  color: #4cd964;
}
.st.wait {
  color: var(--label-3);
}
.st.fail {
  color: #ff6961;
}
.spd {
  font-size: 11.5px;
  color: var(--label-3);
  margin-left: auto;
  font-variant-numeric: tabular-nums;
}
.l2 {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}
.l2 .prog {
  flex: 1;
}
.bytes {
  font-size: 11px;
  color: var(--label-3);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.errmsg {
  font-size: 11.5px;
  color: #ff8a80;
  margin-top: 7px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ops {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}
.opbtn {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  border: none;
  background: var(--fill-2);
  color: var(--label-2);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: background 0.2s;
}
.opbtn:hover {
  background: var(--fill-3);
  color: var(--label);
}
.opbtn.x:hover {
  background: rgba(255, 69, 58, 0.2);
  color: #ff6961;
}

@media (max-width: 640px) {
  .task {
    flex-wrap: wrap;
  }
  .ops {
    margin-left: 54px;
  }
}
@media (max-width: 900px) {
  .page-wrap {
    padding: 18px 18px 150px;
  }
}
</style>
