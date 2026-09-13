<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { Search, LayoutGrid, List, X, Trash2, CheckSquare } from "lucide-vue-next";
import { useWorksStore } from "../../stores/works";

const store = useWorksStore();
const viewMode = defineModel<"grid" | "list">({ default: "grid" });
const keyword = ref("");

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
  <div class="flex flex-col gap-2.5 mb-4">
    <!-- 搜索 + 视图切换 -->
    <div class="flex items-center gap-2">
      <div class="relative flex-1 max-w-md">
        <Search :size="15" class="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
        <input
          v-model="keyword"
          class="input !pl-9 w-full"
          placeholder="搜索标题 / RJ号 / 社团..."
        />
        <button
          v-if="keyword"
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted hover:text-white"
          @click="clearSearch"
        >
          <X :size="14" />
        </button>
      </div>

      <div class="flex-1" />

      <!-- 排序 -->
      <select v-model="store.sortBy" class="input !py-1.5 text-xs" @change="store.applyFilter()">
        <option v-for="s in sorts" :key="s.value" :value="s.value">{{ s.label }}</option>
      </select>

      <!-- 视图切换 -->
      <div class="flex rounded-lg border border-bg-border overflow-hidden">
        <button
          class="px-2.5 py-1.5 transition-colors"
          :class="viewMode === 'grid' ? 'bg-accent/20 text-accent-light' : 'text-muted hover:text-white'"
          @click="viewMode = 'grid'"
        >
          <LayoutGrid :size="15" />
        </button>
        <button
          class="px-2.5 py-1.5 transition-colors"
          :class="viewMode === 'list' ? 'bg-accent/20 text-accent-light' : 'text-muted hover:text-white'"
          @click="viewMode = 'list'"
        >
          <List :size="15" />
        </button>
      </div>

      <!-- 选择模式 -->
      <button
        class="btn-outline !px-2.5 !py-1.5 text-xs"
        :class="store.selectionMode ? 'bg-accent/20 text-accent-light border-accent/40' : ''"
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
      class="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent/10 border border-accent/30 animate-fade-in"
    >
      <button class="btn-ghost !text-xs" @click="store.toggleSelectAll()">
        {{ allSelected ? "取消全选" : "全选本页" }}
      </button>
      <span class="text-[11px] text-muted">已选 {{ store.selectedIds.size }} 项</span>
      <div class="flex-1" />
      <button
        class="btn-outline !text-[11px] !px-2.5 !py-1 text-accent hover:!bg-accent/10"
        :disabled="!store.selectedIds.size"
        @click="store.batchDelete()"
      >
        <Trash2 :size="12" />
        批量删除 ({{ store.selectedIds.size }})
      </button>
      <button class="btn-ghost !text-[11px]" @click="store.toggleSelectionMode()">退出</button>
    </div>

    <!-- 状态筛选 -->
    <div class="flex items-center gap-1">
      <button
        v-for="s in [
          { v: 'all', label: '全部' },
          { v: 'downloaded', label: '已下载' },
          { v: 'not_downloaded', label: '未下载' },
          { v: 'downloading', label: '下载中' },
        ]"
        :key="s.v"
        class="px-3 py-1 rounded-full text-xs transition-colors"
        :class="store.status === s.v ? 'bg-accent text-white' : 'text-muted hover:text-white hover:bg-bg-hover'"
        @click="store.status = s.v; store.applyFilter()"
      >
        {{ s.label }}
      </button>
      <span v-if="store.total > 0" class="ml-auto text-[11px] text-muted">
        共 {{ store.total }} 部作品
      </span>
    </div>
  </div>
</template>
