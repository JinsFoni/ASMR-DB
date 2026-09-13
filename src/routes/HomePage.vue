<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Loader2, ArrowRight } from "lucide-vue-next";
import { useWorksStore } from "../stores/works";
import WorkFilters from "../components/works/WorkFilters.vue";
import WorksGrid from "../components/works/WorksGrid.vue";
import WorksCover from "../components/works/WorksCover.vue";
import WorksList from "../components/works/WorksList.vue";
import EmptyState from "../components/common/EmptyState.vue";
import Pagination from "../components/common/Pagination.vue";

const store = useWorksStore();
const viewMode = ref<"grid" | "cover" | "list">("grid");

// 两种卡片网格的列数联动计算：海报列数取横版列数的理想比例（约 0.585 倍宽），
// 海报图片区高度直接钉在横版图片区高度公式上，保证任意窗口宽度下卡片总高一致
const pageRoot = ref<HTMLElement | null>(null);
const coverCols = ref(3);
const posterCols = ref(5);
const posterImgH = ref(251);

const GAP = 12;
function recalcColumns(width: number) {
  if (width <= 0) return;
  const coverN = Math.max(1, Math.floor((width + GAP) / (300 + GAP)));
  const coverW = (width - GAP * (coverN - 1)) / coverN;
  const idealPosterW = 0.585 * coverW;
  let posterN = Math.max(1, Math.floor((width + GAP) / (idealPosterW + GAP)));
  const widthOf = (n: number) => (width - GAP * (n - 1)) / n;
  if (Math.abs(widthOf(posterN + 1) - idealPosterW) < Math.abs(widthOf(posterN) - idealPosterW)) {
    posterN += 1;
  }
  coverCols.value = coverN;
  posterCols.value = posterN;
  posterImgH.value = 0.75 * coverW;
}

onMounted(() => {
  if (!store.works.length) store.fetchWorks();
  const el = pageRoot.value;
  if (!el) return;
  // 内容区宽度 = clientWidth 减去左右 padding（p-5）
  const contentWidth = () => {
    const cs = getComputedStyle(el);
    return el.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
  };
  const ro = new ResizeObserver((entries) => recalcColumns(entries[0].contentRect.width));
  ro.observe(el);
  // 备用触发源：部分嵌入式环境 ResizeObserver 投递不及时，窗口 resize 事件兜底
  window.addEventListener("resize", () => recalcColumns(contentWidth()));
});
</script>

<template>
  <div ref="pageRoot" class="p-5">
    <WorkFilters v-model="viewMode" />

    <div v-if="store.loading && !store.works.length" class="flex items-center justify-center py-24 text-muted">
      <Loader2 :size="22" class="animate-spin" />
    </div>

    <div v-else-if="store.error" class="text-center py-20 text-accent text-sm">
      {{ store.error }}
    </div>

    <template v-else-if="store.works.length">
      <WorksGrid
        v-if="viewMode === 'grid'"
        :works="store.works"
        :columns="posterCols"
        :image-height="posterImgH"
      />
      <WorksCover v-else-if="viewMode === 'cover'" :works="store.works" :columns="coverCols" />
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
