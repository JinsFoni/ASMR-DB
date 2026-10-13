<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useWorkDetail } from "../composables/useWorkDetail";
import WorkDetailView from "../components/detail/WorkDetailView.vue";

const route = useRoute();
const router = useRouter();

const id = computed(() => String(route.params.rj || "").toUpperCase());

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
} = useWorkDetail({ source: "dlsite", id });

function openDlsite() {
  const url =
    view.value?.work.dlsite_url ??
    (rjCode.value
      ? `https://www.dlsite.com/maniax/work/=/product_id/${rjCode.value}.html`
      : null);
  if (url) window.open(url, "_blank", "noopener");
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
    error-link-label="打开 DLsite"
    retry-label="重试"
    @retry="reload()"
    @tags-updated="onTagsUpdated()"
    @back="router.back()"
    @action="runAction"
    @copy-rj="copyRj()"
    @file-tree-download="onFileTreeDownload"
    @folder-picked="onFolderPicked"
    @error-link="openDlsite"
  />
</template>