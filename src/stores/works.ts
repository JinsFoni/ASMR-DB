import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { WorkView, Tag, Group } from "../lib/types";
import * as api from "../lib/api";

export const useWorksStore = defineStore("works", () => {
  const works = ref<WorkView[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Filters
  const search = ref("");
  const status = ref("all");
  const tagId = ref<number | null>(null);
  const groupId = ref<number | null>(null);
  const sortBy = ref("id");
  const page = ref(1);
  const perPage = ref(48);

  const tags = ref<Tag[]>([]);
  const groups = ref<Group[]>([]);

  // 选择模式 / 批量删除
  const selectionMode = ref(false);
  const selectedIds = ref<Set<number>>(new Set());

  const pages = computed(() => Math.max(1, Math.ceil(total.value / perPage.value)));

  async function loadTags() {
    tags.value = await api.listTags();
  }

  async function loadGroups() {
    groups.value = await api.listGroups();
  }

  async function fetchWorks() {
    loading.value = true;
    error.value = null;
    try {
      const res = await api.listWorks({
        search: search.value || undefined,
        status: status.value !== "all" ? status.value : undefined,
        tagId: tagId.value ?? undefined,
        groupId: groupId.value ?? undefined,
        sortBy: sortBy.value,
        page: page.value,
        perPage: perPage.value,
      });
      works.value = res.items;
      total.value = res.total;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function applyFilter() {
    page.value = 1;
    selectedIds.value = new Set();
    fetchWorks();
  }

  function removeWork(id: number) {
    works.value = works.value.filter((w) => w.work.id !== id);
    total.value = Math.max(0, total.value - 1);
    selectedIds.value.delete(id);
  }

  function upsertWork(view: WorkView) {
    const idx = works.value.findIndex((w) => w.work.id === view.work.id);
    if (idx >= 0) works.value[idx] = view;
    else works.value.unshift(view);
  }

  // ---------- 选择 / 批量删除 ----------

  function toggleSelectionMode() {
    selectionMode.value = !selectionMode.value;
    if (!selectionMode.value) selectedIds.value = new Set();
  }

  function toggleSelect(id: number) {
    const set = new Set(selectedIds.value);
    if (set.has(id)) set.delete(id);
    else set.add(id);
    selectedIds.value = set;
  }

  function toggleSelectAll() {
    if (works.value.length > 0 && selectedIds.value.size === works.value.length) {
      selectedIds.value = new Set();
    } else {
      selectedIds.value = new Set(works.value.map((w) => w.work.id));
    }
  }

  function clearSelection() {
    selectedIds.value = new Set();
  }

  async function batchDelete() {
    const ids = [...selectedIds.value];
    if (!ids.length) return;
    if (
      !confirm(
        `确定删除选中的 ${ids.length} 部作品？\n（仅删除软件内数据，不会删除本地文件）`
      )
    ) {
      return;
    }
    try {
      await api.deleteWorks(ids);
      clearSelection();
      await fetchWorks();
    } catch (e) {
      alert(`批量删除失败: ${e}`);
    }
  }

  return {
    works,
    total,
    loading,
    error,
    search,
    status,
    tagId,
    groupId,
    sortBy,
    page,
    perPage,
    pages,
    tags,
    groups,
    selectionMode,
    selectedIds,
    loadTags,
    loadGroups,
    fetchWorks,
    applyFilter,
    removeWork,
    upsertWork,
    toggleSelectionMode,
    toggleSelect,
    toggleSelectAll,
    clearSelection,
    batchDelete,
  };
});
