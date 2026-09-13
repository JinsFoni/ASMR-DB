<script setup lang="ts">
import { ref, onMounted } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import {
  Library,
  Download,
  Settings,
  Plus,
  Headphones,
  Trash2,
} from "lucide-vue-next";
import { useWorksStore } from "../../stores/works";
import * as api from "../../lib/api";
import type { Tag, Group } from "../../lib/types";
import { APP_VERSION } from "../../lib/constants";

const route = useRoute();
const router = useRouter();
const worksStore = useWorksStore();

const allTags = ref<Tag[]>([]);
const showDeleteTag = ref(false);
const showDeleteGroup = ref(false);

onMounted(async () => {
  allTags.value = await api.listTags();
  worksStore.loadTags();
});

async function deleteTag(tagId: number) {
  if (!confirm("删除标签？(仅移除标签本身)")) return;
  await api.deleteTag(tagId);
  allTags.value = allTags.value.filter((t) => t.id !== tagId);
  worksStore.loadTags();
  if (worksStore.tagId === tagId) {
    worksStore.tagId = null;
    worksStore.applyFilter();
  }
}

async function deleteGroup(group: Group) {
  if (!confirm(`删除分组「${group.name}」？作品不会受影响，仅移除分组。`)) return;
  await api.deleteGroup(group.id);
  worksStore.groups = worksStore.groups.filter((g) => g.id !== group.id);
  if (worksStore.groupId === group.id) {
    worksStore.groupId = null;
    worksStore.applyFilter();
  }
}

function onTagClick(tagId: number) {
  if (showDeleteTag.value) {
    deleteTag(tagId);
    return;
  }
  worksStore.tagId = worksStore.tagId === tagId ? null : tagId;
  goWorksAndFilter();
}

function onGroupClick(groupId: number) {
  if (showDeleteGroup.value) {
    const g = worksStore.groups.find((x) => x.id === groupId);
    if (g) deleteGroup(g);
    return;
  }
  worksStore.groupId = worksStore.groupId === groupId ? null : groupId;
  goWorksAndFilter();
}

/** 不在作品库时跳转到作品库，然后应用筛选。避免重复导航导致的空白页。 */
function goWorksAndFilter() {
  if (route.name !== "home") {
    router.push({ name: "home" }).catch(() => {});
  }
  worksStore.applyFilter();
}

/** 状态筛选：与标签/分组一致，不在作品库时先跳回作品库再应用筛选。 */
function onStatusClick(status: string) {
  worksStore.status = status;
  goWorksAndFilter();
}
</script>

<template>
  <aside class="w-52 shrink-0 flex flex-col bg-bg-deep border-r border-bg-border">
    <div class="flex items-center gap-2.5 px-4 py-4">
      <div class="w-8 h-8 rounded-lg bg-accent/90 flex items-center justify-center text-white">
        <Headphones :size="18" />
      </div>
      <div>
        <div class="text-sm font-semibold text-white leading-tight">DLsite Manager</div>
        <div class="text-[10px] text-muted">ASMR 作品管理</div>
      </div>
    </div>

    <nav class="flex-1 px-2 space-y-0.5 overflow-y-auto">
      <RouterLink
        to="/"
        class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors"
        :class="route.path === '/' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
      >
        <Library :size="16" /> 作品库
      </RouterLink>

      <RouterLink
        to="/import"
        class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors"
        :class="route.path === '/import' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
      >
        <Plus :size="16" /> 导入作品
      </RouterLink>

      <RouterLink
        to="/downloads"
        class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors"
        :class="route.path === '/downloads' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
      >
        <Download :size="16" /> 下载管理
      </RouterLink>

      <RouterLink
        to="/settings"
        class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors"
        :class="route.path === '/settings' ? 'bg-accent/15 text-accent-light' : 'text-muted hover:text-white hover:bg-bg-hover'"
      >
        <Settings :size="16" /> 设置
      </RouterLink>

      <div class="pt-4 pb-1 px-3 text-[11px] font-medium text-muted/70 tracking-wider">
        筛选 · 标签
      </div>
      <button
        class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-sm transition-colors text-left"
        :class="worksStore.status === 'all' ? 'bg-bg-hover text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
        @click="onStatusClick('all')"
      >
        <Library :size="15" /> 全部
      </button>
      <button
        class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-sm transition-colors text-left"
        :class="worksStore.status === 'downloaded' ? 'bg-bg-hover text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
        @click="onStatusClick('downloaded')"
      >
        <span class="w-2 h-2 rounded-full bg-green-400" /> 已下载
      </button>
      <button
        class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-sm transition-colors text-left"
        :class="worksStore.status === 'not_downloaded' ? 'bg-bg-hover text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
        @click="onStatusClick('not_downloaded')"
      >
        <span class="w-2 h-2 rounded-full bg-accent" /> 未下载
      </button>

      <div class="mt-3 space-y-0.5">
        <div class="px-3 pb-1 flex items-center justify-between">
          <span class="text-[11px] font-medium text-muted/70 tracking-wider">标签</span>
          <button class="text-muted/60 hover:text-white" title="切换删除模式" @click="showDeleteTag = !showDeleteTag">
            <Trash2 :size="12" />
          </button>
        </div>
        <button
          v-for="t in allTags"
          :key="t.id"
          class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-sm transition-colors text-left"
          :class="worksStore.tagId === t.id ? 'bg-bg-hover text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
          @click="onTagClick(t.id)"
        >
          <span class="w-2 h-2 rounded-full" :style="{ background: t.color }" />
          <span class="flex-1 ellipsis-1">{{ t.name }}</span>
          <span v-if="showDeleteTag" class="text-accent">×</span>
        </button>
      </div>

      <!-- 分组筛选 -->
      <div class="mt-3 space-y-0.5">
        <div class="px-3 pb-1 flex items-center justify-between">
          <span class="text-[11px] font-medium text-muted/70 tracking-wider">分组</span>
          <button
            class="text-muted/60 hover:text-white"
            title="切换删除模式"
            @click="showDeleteGroup = !showDeleteGroup"
          >
            <Trash2 :size="12" />
          </button>
        </div>
        <button
          v-for="g in worksStore.groups"
          :key="g.id"
          class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-sm transition-colors text-left"
          :class="worksStore.groupId === g.id ? 'bg-bg-hover text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
          @click="onGroupClick(g.id)"
        >
          <span class="w-2 h-2 rounded-full" :style="{ background: g.color }" />
          <span class="flex-1 ellipsis-1">{{ g.name }}</span>
          <span v-if="showDeleteGroup" class="text-accent">×</span>
        </button>
        <div v-if="!worksStore.groups.length" class="px-3 py-1 text-[11px] text-muted/50">
          暂无分组（扫描文件夹时可选智能匹配）
        </div>
      </div>
    </nav>

    <div class="px-4 py-3 border-t border-bg-border text-[10px] text-muted/60">
      v{{ APP_VERSION }} · 小玥喵 🐱
    </div>
  </aside>
</template>
