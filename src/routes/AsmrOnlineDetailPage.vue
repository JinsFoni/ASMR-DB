<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useWorkDetail } from "../composables/useWorkDetail";
import WorkDetailView from "../components/detail/WorkDetailView.vue";

const route = useRoute();
const router = useRouter();

/** 详情→详情 跳转时组件复用，必须响应式 */
const id = computed(() => String(route.params.id || ""));

const {
  loading,
  error,
  view,
  title,
  subtitle,
  rjCode,
  coverUrl,
  statusBadge,
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
  reload,
  onTagsUpdated,
  runAction,
  copiedReset,
  onFileTreeDownload,
  onFolderPicked,
} = useWorkDetail({ source: "asmr", id });

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
    online-tracks
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
    @copy-rj="copiedReset()"
    @file-tree-download="onFileTreeDownload"
    @folder-picked="onFolderPicked"
    @error-link="openAsmrOne"
  />
</template>