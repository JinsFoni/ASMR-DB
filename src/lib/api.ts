import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Work,
  WorkView,
  Tag,
  Group,
  Track,
  PlayHistory,
  ScrapedWork,
  DownloadTask,
  DownloadProgressEvent,
  ScannedItem,
  WorkListResponse,
  AsmrTreeNode,
  AsmrDownloadFile,
} from "./types";

// ---------- Works ----------

export function listWorks(params: {
  search?: string;
  status?: string;
  tagId?: number;
  groupId?: number;
  sortBy?: string;
  page?: number;
  perPage?: number;
}): Promise<WorkListResponse> {
  return invoke("list_works", { q: params });
}

export function getWork(id: number): Promise<WorkView | null> {
  return invoke("get_work", { id });
}

export function updateWork(work: Work): Promise<void> {
  return invoke("update_work", { work });
}

export function deleteWork(id: number): Promise<void> {
  return invoke("delete_work", { id });
}

export function deleteWorks(ids: number[]): Promise<void> {
  return invoke("delete_works", { ids });
}

// ---------- Groups ----------

export function listGroups(): Promise<Group[]> {
  return invoke("list_groups");
}

export function createGroup(name: string, color?: string): Promise<number> {
  return invoke("create_group", { name, color });
}

export function setWorkGroup(workId: number, groupId: number | null): Promise<void> {
  return invoke("set_work_group", { workId, groupId });
}

export function deleteGroup(groupId: number): Promise<void> {
  return invoke("delete_group", { groupId });
}

// ---------- Tags ----------

export function listTags(): Promise<Tag[]> {
  return invoke("list_tags");
}

export function createTag(name: string, color: string): Promise<number> {
  return invoke("create_tag", { name, color });
}

export function assignTag(workId: number, tagId: number): Promise<void> {
  return invoke("assign_tag", { workId, tagId });
}

export function unassignTag(workId: number, tagId: number): Promise<void> {
  return invoke("unassign_tag", { workId, tagId });
}

export function deleteTag(tagId: number): Promise<void> {
  return invoke("delete_tag", { tagId });
}

// ---------- Tracks & history ----------

export function listTracks(workId: number): Promise<Track[]> {
  return invoke("list_tracks", { workId });
}

export function savePlayProgress(
  workId: number,
  trackId: number | null,
  position: number
): Promise<void> {
  return invoke("save_play_progress", { workId, trackId, position });
}

export function getPlayProgress(workId: number): Promise<PlayHistory | null> {
  return invoke("get_play_progress", { workId });
}

// ---------- asmr.one online playback ----------

/**
 * 拉取 asmr.one 上某作品的音频文件列表，返回临时 Track 数组用于在线播放。
 * Track.file_path 形如 `http://asmr.localhost/file/{id}`，由 Rust 侧 asmr URI scheme
 * protocol 拦截转发（带 Bearer token + Range）。
 */
export function listAsmroneTracks(rjCode: string): Promise<Track[]> {
  return invoke("list_asmrone_tracks", { rjCode });
}

export function listAsmroneTree(rjCode: string): Promise<AsmrTreeNode[]> {
  return invoke("list_asmrone_tree", { rjCode });
}

export function downloadAsmroneFiles(
  workId: number,
  files: AsmrDownloadFile[]
): Promise<void> {
  return invoke("download_asmrone_files", { workId, files });
}

/**
 * 构造 asmr.one 文件的自定义协议 URL。
 * Windows 形态为 `http://asmr.localhost/...`；若以后跨平台，macOS/Linux 需改为
 * `asmr://localhost/...`。
 */
export function asmrFileUrl(fileId: number): string {
  return `http://asmr.localhost/file/${fileId}`;
}

// ---------- Import ----------

export function importByRj(
  rjCode: string,
  source?: string
): Promise<{ ok: boolean; work: WorkView }> {
  return invoke("import_by_rj", { rjCode, source });
}

export function previewByRj(rjCode: string, source?: string): Promise<ScrapedWork> {
  return invoke("preview_by_rj", { rjCode, source });
}

export function scanFolder(folder: string): Promise<{ items: ScannedItem[]; count: number }> {
  return invoke("scan_folder", { folder, async: true });
}

export function bindLocalFolder(
  rjCode: string,
  path: string,
  groupName?: string | null
): Promise<{ ok: boolean; trackCount: number; groupName?: string | null }> {
  return invoke("bind_local_folder", { rjCode, path, groupName });
}

export function scanAudioTracks(workId: number): Promise<{ ok: boolean; count: number; message?: string }> {
  return invoke("scan_audio_tracks", { workId });
}

export function openAudioFile(path: string): Promise<void> {
  return invoke("open_audio_file", { path });
}

export function findSubtitleForTrack(
  trackPath: string
): Promise<{ name: string; content: string } | null> {
  return invoke("find_subtitle_for_track", { trackPath });
}

export function fetchSubtitleContent(url: string): Promise<string> {
  return invoke("fetch_subtitle_content", { url });
}

// ---------- Download ----------

export function downloadWork(workId: number, url: string): Promise<void> {
  return invoke("download_work", { workId, url });
}

export function pauseDownload(rjCode: string): Promise<void> {
  return invoke("pause_download", { rjCode });
}

export function resumeDownload(rjCode: string): Promise<void> {
  return invoke("resume_download", { rjCode });
}

export function cancelDownload(rjCode: string): Promise<void> {
  return invoke("cancel_download", { rjCode });
}

export function getDownloadQueue(): Promise<DownloadTask[]> {
  return invoke("get_download_queue");
}

export function setDownloadSettings(
  downloadDir: string,
  concurrency: number
): Promise<void> {
  return invoke("set_download_settings", { downloadDir, concurrency });
}

export function getDownloadSettings(): Promise<{
  download_dir: string;
  concurrency: number;
}> {
  return invoke("get_download_settings");
}

// ---------- Settings ----------

export function getSettings(): Promise<{
  download_dir: string | null;
  theme: string;
  asmr_one_token: string | null;
}> {
  return invoke("get_settings");
}

export function setAsmrToken(token: string): Promise<void> {
  return invoke("set_asmr_token", { token });
}

export function clearAsmrToken(): Promise<void> {
  return invoke("clear_asmr_token");
}

export function setSettings(
  theme?: string,
  downloadDir?: string
): Promise<void> {
  return invoke("set_settings", { theme, downloadDir });
}

export function getDbPath(): Promise<string> {
  return invoke("get_db_path");
}

export function exportDatabase(dest: string): Promise<string> {
  return invoke("export_database", { dest });
}

export function importDatabase(source: string): Promise<string> {
  return invoke("import_database", { source });
}

export function openFolder(path: string): Promise<void> {
  return invoke("open_folder", { path });
}

// ---------- Events ----------

export async function onDownloadProgress(
  handler: (ev: DownloadProgressEvent) => void
): Promise<() => void> {
  const unlisten = await listen<DownloadProgressEvent>("download-progress", (e) => {
    handler(e.payload);
  });
  return unlisten;
}
