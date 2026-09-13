<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Plus, X } from "lucide-vue-next";
import type { Tag } from "../../lib/types";
import * as api from "../../lib/api";
import { useWorksStore } from "../../stores/works";

const props = defineProps<{ workId: number; tags: Tag[] }>();
const emit = defineEmits<{ updated: [] }>();

const worksStore = useWorksStore();
const allTags = ref<Tag[]>([]);
const adding = ref(false);
const newName = ref("");

const COLORS = ["#e94560", "#4e9bff", "#45d483", "#b45eff", "#ff9f43", "#888888"];

onMounted(async () => {
  allTags.value = await api.listTags();
});

async function toggleTag(tag: Tag) {
  const has = props.tags.some((t) => t.id === tag.id);
  if (has) await api.unassignTag(props.workId, tag.id);
  else await api.assignTag(props.workId, tag.id);
  emit("updated");
}

async function createAndAssign() {
  const name = newName.value.trim();
  if (!name) return;
  const color = COLORS[Math.floor(Math.random() * COLORS.length)];
  const id = await api.createTag(name, color);
  await api.assignTag(props.workId, id);
  newName.value = "";
  adding.value = false;
  allTags.value = await api.listTags();
  worksStore.loadTags();
  emit("updated");
}
</script>

<template>
  <div class="flex flex-wrap gap-1.5 items-center">
    <span
      v-for="t in tags"
      :key="t.id"
      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] border cursor-pointer hover:opacity-80"
      :style="{ color: t.color, borderColor: t.color + '66', background: t.color + '14' }"
      :title="'点击移除「' + t.name + '」'"
      @click="toggleTag(t)"
    >
      {{ t.name }}
      <X :size="10" class="opacity-70" />
    </span>

    <!-- 已有标签下拉 -->
    <button
      v-if="!adding"
      class="inline-flex items-center gap-0.5 px-2 py-0.5 rounded-full text-[11px] text-muted hover:text-white hover:bg-bg-hover border border-dashed border-bg-border"
      @click="adding = true"
    >
      <Plus :size="11" /> 添加标签
    </button>

    <div v-else class="flex items-center gap-1">
      <input
        v-model="newName"
        class="input !py-0.5 !text-[11px] !px-2 w-28"
        placeholder="新标签名"
        @keyup.enter="createAndAssign"
        @keyup.esc="adding = false"
      />
      <button class="btn-primary !text-[11px] !px-2 !py-1" @click="createAndAssign">添加</button>
      <button class="btn-ghost !text-[11px] !px-1.5 !py-1" @click="adding = false">取消</button>
    </div>

    <!-- 可选已有标签 -->
    <template v-if="adding">
      <button
        v-for="t in allTags.filter((t) => !tags.some((x) => x.id === t.id))"
        :key="t.id"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] border hover:opacity-80"
        :style="{ color: t.color, borderColor: t.color + '66', background: t.color + '14' }"
        @click="toggleTag(t)"
      >
        + {{ t.name }}
      </button>
    </template>
  </div>
</template>
