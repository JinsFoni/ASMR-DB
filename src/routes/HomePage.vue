<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Loader2, ArrowRight } from "lucide-vue-next";
import { useWorksStore } from "../stores/works";
import WorkFilters from "../components/works/WorkFilters.vue";
import WorksGrid from "../components/works/WorksGrid.vue";
import WorksList from "../components/works/WorksList.vue";
import EmptyState from "../components/common/EmptyState.vue";
import Pagination from "../components/common/Pagination.vue";

const store = useWorksStore();
const viewMode = ref<"grid" | "list">("grid");

onMounted(() => {
  if (!store.works.length) store.fetchWorks();
});
</script>

<template>
  <div class="p-5">
    <WorkFilters v-model="viewMode" />

    <div v-if="store.loading && !store.works.length" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="22" class="animate-spin" />
    </div>

    <div v-else-if="store.error" class="text-center py-20 text-accent text-sm">
      {{ store.error }}
    </div>

    <template v-else-if="store.works.length">
      <WorksGrid v-if="viewMode === 'grid'" :works="store.works" />
      <WorksList v-else :works="store.works" />
      <Pagination
        v-if="store.pages > 1"
        :page="store.page"
        :total="store.pages"
        @change="store.page = $event; store.fetchWorks()"
      />
    </template>

    <EmptyState
      v-else
      title="作品库是空的"
      desc="点击下方按钮，用 RJ 号导入你的第一部作品吧！"
    >
      <RouterLink to="/import" class="btn-primary mt-4">
        导入作品 <ArrowRight :size="14" />
      </RouterLink>
    </EmptyState>
  </div>
</template>
