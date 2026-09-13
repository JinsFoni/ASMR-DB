<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ArrowLeft,
  Globe,
  Monitor,
  Copy,
  Check,
  Loader2,
  ImageOff,
  BookmarkCheck,
  RotateCw,
} from "lucide-vue-next";
import * as api from "../lib/api";
import type { ScrapedWork } from "../lib/types";
import { useWorksStore } from "../stores/works";

const route = useRoute();
const router = useRouter();
const worksStore = useWorksStore();
const rj = String(route.params.rj || "").toUpperCase();

const scraped = ref<ScrapedWork | null>(null);
const loading = ref(true);
const error = ref("");
const copied = ref(false);
const importing = ref(false);

const title = computed(
  () => scraped.value?.title_zh || scraped.value?.title_ja || scraped.value?.rj_code
);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    scraped.value = await api.previewByRj(rj);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

async function copyRj() {
  await navigator.clipboard.writeText(rj);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}

/** 入库：抓取元数据保存后跳转本地作品详情 */
async function importWork() {
  if (!scraped.value || importing.value) return;
  importing.value = true;
  try {
    const res = await api.importByRj(rj);
    // 同步 Pinia store，否则切回作品库时因缓存不会重新拉取，看不到新入库的作品
    worksStore.fetchWorks();
    worksStore.loadTags();
    router.push(`/work/${res.work.work.id}`);
  } catch (e) {
    alert(`入库失败: ${e}`);
    importing.value = false;
  }
}

function openDlsite() {
  const url =
    scraped.value?.dlsite_url ||
    `https://www.dlsite.com/maniax/work/=/product_id/${rj}.html`;
  window.open(url, "_blank", "noopener");
}

function openAsmrOne() {
  window.open(`https://www.asmr.one/work/${rj}`, "_blank", "noopener");
}
</script>

<template>
  <div v-if="loading" class="flex items-center justify-center py-24 text-muted">
    <Loader2 :size="24" class="animate-spin" />
  </div>

  <div v-else-if="error" class="p-5 max-w-5xl mx-auto space-y-4">
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="router.back()">
      <ArrowLeft :size="14" /> 返回
    </button>
    <div class="card p-6 text-sm text-muted space-y-3">
      <div>⚠️ 在线抓取作品数据失败（{{ rj }}）</div>
      <div class="text-xs">{{ error }}</div>
      <div class="flex gap-2">
        <button class="btn-outline !text-xs" :disabled="loading" @click="load">
          <RotateCw :size="13" /> 重试
        </button>
        <button class="btn-ghost !text-xs" @click="openDlsite">
          <Globe :size="13" /> 打开 DLsite 页面
        </button>
      </div>
    </div>
  </div>

  <div v-else-if="scraped" class="p-5 space-y-5 max-w-5xl mx-auto">
    <!-- 返回 -->
    <button class="btn-ghost !px-2 !py-1 text-xs" @click="router.back()">
      <ArrowLeft :size="14" /> 返回
    </button>

    <div class="flex gap-5">
      <!-- 封面（完整封面不裁切，与横版卡片一致；加宽并垂直居中避免信息列下方大片空白） -->
      <div class="w-72 shrink-0 self-center rounded-xl overflow-hidden bg-bg-hover border border-bg-border">
        <img v-if="scraped.cover_url" :src="scraped.cover_url" class="w-full h-auto" />
        <div v-else class="w-72 aspect-[4/3] flex items-center justify-center text-muted/40">
          <ImageOff :size="40" />
        </div>
      </div>

      <!-- 信息 -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 text-xs">
          <span class="font-mono text-muted">{{ rj }}</span>
          <button class="text-muted hover:text-white" title="复制RJ号" @click="copyRj">
            <Check v-if="copied" :size="13" class="text-green-400" />
            <Copy v-else :size="13" />
          </button>
          <span class="px-1.5 py-0.5 rounded text-[10px] border border-yellow-500/50 text-yellow-400 bg-yellow-500/10">
            未入库
          </span>
        </div>

        <h1 class="text-xl font-bold text-white mt-2 leading-snug">{{ title }}</h1>
        <div v-if="scraped.title_ja && scraped.title_ja !== title" class="text-sm text-muted mt-0.5">
          {{ scraped.title_ja }}
        </div>

        <!-- 元数据行 -->
        <div class="flex flex-wrap gap-x-5 gap-y-1.5 mt-4 text-xs text-muted">
          <span v-if="scraped.circle_name">🏢 社团：{{ scraped.circle_name }}</span>
          <span v-if="scraped.sale_date">📅 发售：{{ scraped.sale_date }}</span>
          <span v-if="scraped.price">💰 {{ scraped.price.toLocaleString() }}円</span>
          <span v-if="scraped.duration_min">⏱ {{ scraped.duration_min }}分</span>
          <span v-if="scraped.file_size_mb">📦 {{ scraped.file_size_mb.toFixed(1) }}MB</span>
          <span v-if="scraped.age_class" class="uppercase">{{ scraped.age_class }}</span>
        </div>

        <div v-if="scraped.actors.length" class="mt-2 text-xs text-muted">
          🎤 声优：<span class="text-white/80">{{ scraped.actors.join(" / ") }}</span>
        </div>

        <!-- 标签（在线预览只读） -->
        <div v-if="scraped.tags.length" class="flex flex-wrap gap-1.5 mt-3">
          <span
            v-for="t in scraped.tags"
            :key="t"
            class="px-2 py-0.5 rounded text-[11px] bg-bg-hover text-muted border border-bg-border"
          >
            {{ t }}
          </span>
        </div>

        <!-- 操作按钮：入库替代删除，本地文件相关操作不显示 -->
        <div class="flex flex-wrap gap-2 mt-5">
          <button class="btn-primary" :disabled="importing" @click="importWork">
            <Loader2 v-if="importing" :size="14" class="animate-spin" />
            <BookmarkCheck v-else :size="14" />
            入库
          </button>
          <button class="btn-ghost" @click="openDlsite">
            <Globe :size="14" /> DLsite 页面
          </button>
          <button class="btn-ghost" @click="openAsmrOne">
            <Monitor :size="14" /> ASMR One
          </button>
        </div>
      </div>
    </div>

    <!-- 简介 -->
    <div v-if="scraped.description" class="card p-4">
      <div class="text-xs font-semibold text-white mb-2">📝 简介</div>
      <p class="text-xs text-muted leading-relaxed whitespace-pre-line max-h-48 overflow-y-auto">
        {{ scraped.description }}
      </p>
    </div>
  </div>
</template>
