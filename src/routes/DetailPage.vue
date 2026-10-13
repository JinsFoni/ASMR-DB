<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useWorkDetail } from "../composables/useWorkDetail";
import WorkDetailView from "../components/detail/WorkDetailView.vue";

const route = useRoute();
const router = useRouter();

/** 路由参数响应式：hash 模式下 详情→详情 跳转会复用组件，必须用 computed */
const id = computed(() => String(route.params.id || ""));

/**
 * 必须解构为顶层绑定。Vue 模板只对**顶层 setup 绑定**自动解包 ref；
 * 写成 `d.title` 传入的是 Ref 对象，会触发 props 类型错误且渲染成 [object Object]。
 */
const {
  loading,
  error,
  view,
  title,
  subtitle,
  rjCode,
  coverUrl,
  statusBadge,
  ageBadge,
  meta,
  actors,
  tags,
  editableTags,
  workId,
  description,
  descriptionNote,
  descriptionLoading,
  actions,
  displayTracks,
  fileTree,
  tracksLoading,
  treeLoading,
  notFound,
  busyKey,
  copied,
  showFileTree,
  showFolderPicker,
  showUrlInput,
  dlUrl,
  mode,
  reload,
  onTagsUpdated,
  runAction,
  copyRj,
  onFileTreeDownload,
  onFolderPicked,
} = useWorkDetail({ source: "work", id });

function openAsmrOne() {
  if (rjCode.value) window.open(`https://www.asmr.one/work/${rjCode.value}`, "_blank", "noopener");
}
</script>

<template>
  <WorkDetailView
    :loading="loading"
    :error="error"
    :title="title"
    :subtitle="subtitle"
    :rj-code="rjCode"
    :cover-url="coverUrl"
    :status-badge="statusBadge"
    :age-badge="ageBadge"
    :meta="meta"
    :actors="actors"
    :tags="tags"
    :editable-tags="editableTags"
    :work-id="workId"
    :description="description"
    :description-note="descriptionNote"
    :description-loading="descriptionLoading"
    :actions="actions"
    :tracks="displayTracks"
    :track-work="view"
    :online-tracks="mode === 'asmr'"
    :tree="fileTree"
    :tracks-loading="tracksLoading"
    :tree-loading="treeLoading"
    :not-found="notFound"
    :busy-key="busyKey"
    :copied="copied"
    v-model:file-tree-visible="showFileTree"
    v-model:folder-picker-visible="showFolderPicker"
    v-model:url-input-visible="showUrlInput"
    v-model:url-input-value="dlUrl"
    @submit-url="runAction('dl-download')"
    error-link-label="打开 ASMR ONE 网页"
    retry-label="重试"
    @retry="reload()"
    @tags-updated="onTagsUpdated()"
    @back="router.back()"
    @action="runAction"
    @copy-rj="copyRj()"
    @file-tree-download="onFileTreeDownload"
    @folder-picked="onFolderPicked"
    @error-link="openAsmrOne"
  />
</template>