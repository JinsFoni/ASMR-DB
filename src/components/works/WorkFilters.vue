<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { Search, LayoutGrid, RectangleHorizontal, List, X, Trash2, CheckSquare } from "lucide-vue-next";
import { useWorksStore } from "../../stores/works";
import * as api from "../../lib/api";

const store = useWorksStore();
const viewMode = defineModel<"grid" | "cover" | "list">({ default: "grid" });
const keyword = ref("");

const showFilters = ref(false);
const showDeleteTag = ref(false);
const showDeleteGroup = ref(false);

onMounted(() => {
  store.loadTags();
});

async function deleteTag(tagId: number) {
  if (!confirm("删除标签？(仅移除标签本身)")) return;
  await api.deleteTag(tagId);
  store.loadTags();
  if (store.tagId === tagId) {
    store.tagId = null;
    store.applyFilter();
  }
}

async function deleteGroup(groupId: number) {
  const g = store.groups.find((x) => x.id === groupId);
  if (!g) return;
  if (!confirm(`删除分组「${g.name}」？作品不会受影响，仅移除分组。`)) return;
  await api.deleteGroup(g.id);
  store.groups = store.groups.filter((x) => x.id !== g.id);
  if (store.groupId === g.id) {
    store.groupId = null;
    store.applyFilter();
  }
}

function onTagClick(tagId: number) {
  if (showDeleteTag.value) {
    deleteTag(tagId);
    return;
  }
  store.tagId = store.tagId === tagId ? null : tagId;
  store.applyFilter();
}

function onGroupClick(groupId: number) {
  if (showDeleteGroup.value) {
    deleteGroup(groupId);
    return;
  }
  store.groupId = store.groupId === groupId ? null : groupId;
  store.applyFilter();
}

const allSelected = computed(
  () => store.works.length > 0 && store.works.every((w) => store.selectedIds.has(w.work.id))
);

let debounce: number | null = null;
watch(keyword, (v) => {
  if (debounce) window.clearTimeout(debounce);
  debounce = window.setTimeout(() => {
    store.search = v.trim();
    store.applyFilter();
  }, 350);
});

function clearSearch() {
  keyword.value = "";
  store.search = "";
  store.applyFilter();
}

const sorts = [
  { value: "id", label: "最近添加" },
  { value: "sale_date", label: "发售日期" },
  { value: "title", label: "标题" },
  { value: "circle", label: "社团" },
];
</script>

<template>
  <div class="flex flex-col gap-3 mb-5">
    <!-- 搜索 + 排序 + 视图切换（设计稿：tools 行） -->
    <div class="flex items-center gap-3 flex-wrap">
      <label class="aura-search">
        <Search :size="15" />
        <input
          v-model="keyword"
          placeholder="搜索标题 / RJ号 / 社团..."
        />
        <button v-if="keyword" class="text-label-3 hover:text-label" @click="clearSearch">
          <X :size="14" />
        </button>
      </label>

      <!-- 排序 -->
      <select v-model="store.sortBy" class="seg-select" @change="store.applyFilter()">
        <option v-for="s in sorts" :key="s.value" :value="s.value">{{ s.label }}</option>
      </select>

      <!-- 视图切换 -->
      <div class="seg">
        <button :class="{ on: viewMode === 'grid' }" title="网格视图" @click="viewMode = 'grid'">
          <LayoutGrid :size="14" />
        </button>
        <button :class="{ on: viewMode === 'cover' }" title="横版封面" @click="viewMode = 'cover'">
          <RectangleHorizontal :size="14" />
        </button>
        <button :class="{ on: viewMode === 'list' }" title="列表视图" @click="viewMode = 'list'">
          <List :size="14" />
        </button>
      </div>

      <!-- 选择模式 -->
      <button
        class="btn ghost sm"
        :class="store.selectionMode ? 'primary' : ''"
        :style="store.selectionMode ? 'background: var(--blue); color: #fff;' : ''"
        title="批量选择 / 删除"
        @click="store.toggleSelectionMode()"
      >
        <CheckSquare :size="14" />
        {{ store.selectionMode ? "退出选择" : "选择" }}
      </button>
    </div>

    <!-- 批量操作栏 -->
    <div
      v-if="store.selectionMode"
      class="flex items-center gap-2 px-4 py-2.5 rounded-2xl animate-fade-in"
      style="background: var(--fill-1)"
    >
      <button class="btn ghost sm" @click="store.toggleSelectAll()">
        {{ allSelected ? "取消全选" : "全选本页" }}
      </button>
      <span class="text-[11px] text-muted">已选 {{ store.selectedIds.size }} 项</span>
      <div class="flex-1" />
      <button
        class="btn danger sm"
        :disabled="!store.selectedIds.size"
        @click="store.batchDelete()"
      >
        <Trash2 :size="12" />
        批量删除 ({{ store.selectedIds.size }})
      </button>
      <button class="btn ghost sm" @click="store.toggleSelectionMode()">退出</button>
    </div>

    <!-- 状态筛选 chips（设计稿：chips 行） -->
    <div class="flex items-center gap-2 flex-wrap filter-anchor">
      <button
        v-for="s in [
          { v: 'all', label: '全部' },
          { v: 'downloaded', label: '已下载' },
          { v: 'not_downloaded', label: '未下载' },
          { v: 'downloading', label: '下载中' },
        ]"
        :key="s.v"
        class="chip"
        :class="{ on: store.status === s.v }"
        @click="store.status = s.v; store.applyFilter()"
      >
        {{ s.label }}
      </button>

      <!-- 筛选弹出：标签 / 分组（原导航栏「筛选」按钮迁移至此） -->
      <button
        class="chip"
        :class="{ on: showFilters }"
        title="筛选：标签 / 分组"
        @click="showFilters = !showFilters"
      >
        筛选
      </button>

      <span v-if="store.total > 0" class="ml-auto text-[12.5px]" style="color: var(--label-3)">
        共 {{ store.total }} 部作品
      </span>

      <!-- 筛选浮层 -->
      <transition name="pop">
        <div v-if="showFilters" class="filter-pop glass">
        <div class="fp-row">
          <span class="fp-label">
            标签
            <button
              class="fp-del"
              :class="{ on: showDeleteTag }"
              title="切换删除模式"
              @click="showDeleteTag = !showDeleteTag"
            >
              <Trash2 :size="11" />
            </button>
          </span>
          <div class="fp-chips">
            <button
              v-for="t in store.tags"
              :key="t.id"
              class="chip sm-chip"
              :class="{ on: store.tagId === t.id }"
              @click="onTagClick(t.id)"
            >
              <span class="w-1.5 h-1.5 rounded-full" :style="{ background: t.color }" />
              {{ t.name }}
              <span v-if="showDeleteTag" class="text-sys-red font-bold">×</span>
            </button>
            <span v-if="!store.tags.length" class="fp-empty">暂无标签</span>
          </div>
        </div>

        <div class="fp-row">
          <span class="fp-label">
            分组
            <button
              class="fp-del"
              :class="{ on: showDeleteGroup }"
              title="切换删除模式"
              @click="showDeleteGroup = !showDeleteGroup"
            >
              <Trash2 :size="11" />
            </button>
          </span>
          <div class="fp-chips">
            <button
              v-for="g in store.groups"
              :key="g.id"
              class="chip sm-chip"
              :class="{ on: store.groupId === g.id }"
              @click="onGroupClick(g.id)"
            >
              <span class="w-1.5 h-1.5 rounded-full" :style="{ background: g.color }" />
              {{ g.name }}
              <span v-if="showDeleteGroup" class="text-sys-red font-bold">×</span>
            </button>
            <span v-if="!store.groups.length" class="fp-empty">暂无分组</span>
          </div>
        </div>
        </div>
      </transition>
    </div>
  </div>
</template>

<style scoped>
.aura-search {
  flex: 1;
  min-width: 220px;
  max-width: 560px;
  display: flex;
  align-items: center;
  gap: 10px;
  height: 48px;
  background: var(--bg-secondary);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.08);
  padding: 0 16px;
  color: var(--label-3);
  transition: box-shadow 0.2s;
  cursor: text;
}
html[data-theme="light"] .aura-search,
html[data-theme="sakura"] .aura-search {
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(0, 0, 0, 0.08);
}
.aura-search:focus-within {
  box-shadow: 0 0 0 4px rgba(47, 143, 239, 0.35), var(--shadow-card);
}
html[data-theme="sakura"] .aura-search:focus-within {
  box-shadow: 0 0 0 4px rgba(236, 72, 153, 0.35), var(--shadow-card);
}
.aura-search input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  font-family: inherit;
  font-size: 14.5px;
  font-weight: 500;
  color: var(--label);
}
.aura-search input::placeholder {
  color: var(--label-3);
}
.aura-search button {
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px;
  display: grid;
  place-items: center;
  color: var(--label-3);
}
.aura-search button:hover {
  color: var(--label);
}
.seg button {
  display: grid;
  place-items: center;
}

/* 筛选浮层：锚定在 chips 行下方 */
.filter-anchor {
  position: relative;
}
.filter-pop {
  position: absolute;
  top: calc(100% + 10px);
  left: 0;
  z-index: 50;
  width: min(680px, calc(100vw - 56px));
  border-radius: var(--r-lg);
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.7),
    inset 0 0 0 0.5px rgba(255, 255, 255, 0.14);
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
html[data-theme="light"] .filter-pop,
html[data-theme="sakura"] .filter-pop {
  box-shadow:
    0 24px 60px -16px rgba(0, 0, 0, 0.2),
    inset 0 0 0 0.5px rgba(0, 0, 0, 0.06);
}
.fp-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
.fp-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: 58px;
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--label-2);
  padding-top: 7px;
}
.fp-del {
  border: none;
  background: none;
  color: var(--label-3);
  cursor: pointer;
  padding: 2px;
  border-radius: 6px;
}
.fp-del:hover,
.fp-del.on {
  color: var(--label);
  background: var(--fill-2);
}
.fp-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  flex: 1;
  min-width: 0;
}
.sm-chip {
  height: 28px;
  padding: 0 12px;
  font-size: 12px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.fp-empty {
  font-size: 11.5px;
  color: var(--label-3);
  padding-top: 6px;
}
.pop-enter-active,
.pop-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}
.pop-enter-from,
.pop-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
