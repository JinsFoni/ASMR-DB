import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { DownloadTask } from "../lib/types";
import * as api from "../lib/api";

export const useDownloadStore = defineStore("download", () => {
  const tasks = ref<DownloadTask[]>([]);
  const settings = ref<{ download_dir: string; concurrency: number }>({
    download_dir: "",
    concurrency: 1,
  });

  const activeCount = computed(
    () => tasks.value.filter((t) => t.status === "downloading" || t.status === "queued").length
  );

  function upsertTask(task: DownloadTask) {
    const idx = tasks.value.findIndex((t) => t.rj_code === task.rj_code);
    if (idx >= 0) tasks.value[idx] = task;
    else tasks.value.unshift(task);
  }

  function handleProgress(ev: {
    rjCode: string;
    status: string;
    progress: number;
    speed: number;
    bytesDone: number;
    bytesTotal: number;
    error: string | null;
  }) {
    upsertTask({
      rj_code: ev.rjCode,
      title: "",
      status: ev.status as DownloadTask["status"],
      progress: ev.progress,
      speed: ev.speed,
      bytes_done: ev.bytesDone,
      bytes_total: ev.bytesTotal,
      error: ev.error,
    });
  }

  async function refresh() {
    tasks.value = await api.getDownloadQueue();
  }

  async function loadSettings() {
    const s = await api.getDownloadSettings();
    settings.value = s;
  }

  async function saveSettings(dir: string, concurrency: number) {
    await api.setDownloadSettings(dir, concurrency);
    settings.value = { download_dir: dir, concurrency };
  }

  async function startDownload(workId: number, url: string) {
    await api.downloadWork(workId, url);
    await refresh();
  }

  return {
    tasks,
    settings,
    activeCount,
    upsertTask,
    handleProgress,
    refresh,
    loadSettings,
    saveSettings,
    startDownload,
  };
});
