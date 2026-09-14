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
  DlsiteRankingResponse,
  AsmrOnlineWork,
  AsmrWorksPage,
  AsmrPlaylist,
} from "./types";

// ============================ HTTP 基础封装 ============================

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(path, init);
  } catch (e) {
    throw new Error(`无法连接服务器，请确认后端已启动（${e}）`);
  }
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const data = await res.json();
      if (data?.error) msg = data.error;
      else if (data?.message) msg = data.message;
    } catch {
      // 非 JSON 错误体
    }
    throw new Error(msg);
  }
  const text = await res.text();
  if (!text) return undefined as T;
  return JSON.parse(text) as T;
}

function get<T>(path: string): Promise<T> {
  return request<T>(path);
}

function post<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body === undefined ? "{}" : JSON.stringify(body),
  });
}

function put<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: body === undefined ? "{}" : JSON.stringify(body),
  });
}

function del<T>(path: string): Promise<T> {
  return request<T>(path, { method: "DELETE" });
}

function qs(params: Record<string, string | number | undefined>): string {
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== "") sp.set(k, String(v));
  }
  const s = sp.toString();
  return s ? `?${s}` : "";
}

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
  return get(
    `/api/works${qs({
      search: params.search,
      status: params.status,
      tagId: params.tagId,
      groupId: params.groupId,
      sortBy: params.sortBy,
      page: params.page,
      perPage: params.perPage,
    })}`
  );
}

export function getWork(id: number): Promise<WorkView | null> {
  return get(`/api/works/${id}`);
}

export function updateWork(work: Work): Promise<void> {
  return put("/api/works", work);
}

export function deleteWork(id: number): Promise<void> {
  return del(`/api/works/${id}`);
}

export function deleteWorks(ids: number[]): Promise<void> {
  return post("/api/works/delete-batch", { ids });
}

// ---------- Groups ----------

export function listGroups(): Promise<Group[]> {
  return get("/api/groups");
}

export function createGroup(name: string, color?: string): Promise<number> {
  return post("/api/groups", { name, color });
}

export function setWorkGroup(workId: number, groupId: number | null): Promise<void> {
  return put(`/api/works/${workId}/group`, { groupId });
}

export function deleteGroup(groupId: number): Promise<void> {
  return del(`/api/groups/${groupId}`);
}

// ---------- Tags ----------

export function listTags(): Promise<Tag[]> {
  return get("/api/tags");
}

export function createTag(name: string, color: string): Promise<number> {
  return post("/api/tags", { name, color });
}

export function assignTag(workId: number, tagId: number): Promise<void> {
  return post(`/api/works/${workId}/tags`, { tagId });
}

export function unassignTag(workId: number, tagId: number): Promise<void> {
  return del(`/api/works/${workId}/tags/${tagId}`);
}

export function deleteTag(tagId: number): Promise<void> {
  return del(`/api/tags/${tagId}`);
}

// ---------- Tracks & history ----------

export function listTracks(workId: number): Promise<Track[]> {
  return get(`/api/works/${workId}/tracks`);
}

export function savePlayProgress(
  workId: number,
  trackId: number | null,
  position: number
): Promise<void> {
  return post(`/api/works/${workId}/progress`, { trackId, position });
}

export function getPlayProgress(workId: number): Promise<PlayHistory | null> {
  return get(`/api/works/${workId}/progress`);
}

// ---------- asmr.one online playback ----------

/**
 * 拉取 asmr.one 上某作品的音频文件列表，返回临时 Track 数组用于在线播放。
 * file_path 为 asmr.one 公开 CDN 流 URL；个别缺 CDN URL 的节点会回退到
 * 本服务的代理路径 /api/asmr/file/{hash}（服务端注入 Token 并透传 Range）。
 */
export function listAsmroneTracks(rjCode: string): Promise<Track[]> {
  return get(`/api/asmr/tracks${qs({ rj: rjCode })}`);
}

export function listAsmroneTree(rjCode: string): Promise<AsmrTreeNode[]> {
  return get(`/api/asmr/tree${qs({ rj: rjCode })}`);
}

export function downloadAsmroneFiles(
  workId: number,
  files: AsmrDownloadFile[]
): Promise<void> {
  return post("/api/download/asmr", { workId, files });
}

/**
 * asmr.one 文件的代理 URL（服务端注入 Token 并转发，前端拿不到 token）。
 */
export function asmrFileUrl(fileId: number | string): string {
  return `/api/asmr/file/${fileId}`;
}

/**
 * 本地音频文件的服务 URL（HTTP Range 流式传输，支持拖动 seek）。
 */
export function localAudioUrl(path: string): string {
  return `/api/audio?path=${encodeURIComponent(path)}`;
}

/**
 * 判断音轨是否为在线播放（asmr.one CDN / 本服务代理路径），
 * 在线音轨不写播放进度，避免 track_id 与本地音轨混淆。
 */
export function isOnlineTrack(filePath: string): boolean {
  return /^https?:\/\//i.test(filePath) || filePath.startsWith("/api/");
}

// ---------- Import ----------

export function importByRj(
  rjCode: string,
  source?: string
): Promise<{ ok: boolean; work: WorkView }> {
  return post("/api/import/by-rj", { rjCode, source });
}

export function previewByRj(rjCode: string, source?: string): Promise<ScrapedWork> {
  return post("/api/import/preview", { rjCode, source });
}

export function scanFolder(folder: string): Promise<{ items: ScannedItem[]; count: number }> {
  return post("/api/import/scan-folder", { folder });
}

export function bindLocalFolder(
  rjCode: string,
  path: string,
  groupName?: string | null
): Promise<{ ok: boolean; trackCount: number; groupName?: string | null }> {
  return post("/api/import/bind-folder", { rjCode, path, groupName });
}

export function scanAudioTracks(workId: number): Promise<{ ok: boolean; count: number; message?: string }> {
  return post(`/api/works/${workId}/scan-tracks`);
}

export function openAudioFile(path: string): Promise<void> {
  return post("/api/settings/open-audio", { path });
}

export function findSubtitleForTrack(
  trackPath: string
): Promise<{ name: string; content: string } | null> {
  return get(`/api/subtitle/find${qs({ path: trackPath })}`);
}

export function fetchSubtitleContent(url: string): Promise<string> {
  return get<{ content: string }>(`/api/subtitle/fetch${qs({ url })}`).then(
    (r) => r.content
  );
}

// ---------- Download ----------

export function downloadWork(workId: number, url: string): Promise<void> {
  return post("/api/download", { workId, url });
}

export function pauseDownload(rjCode: string): Promise<void> {
  return post("/api/download/pause", { rjCode });
}

export function resumeDownload(rjCode: string): Promise<void> {
  return post("/api/download/resume", { rjCode });
}

export function cancelDownload(rjCode: string): Promise<void> {
  return post("/api/download/cancel", { rjCode });
}

export function getDownloadQueue(): Promise<DownloadTask[]> {
  return get("/api/download/queue");
}

export function setDownloadSettings(
  downloadDir: string,
  concurrency: number
): Promise<void> {
  return post("/api/download/settings", { downloadDir, concurrency });
}

export function getDownloadSettings(): Promise<{
  download_dir: string;
  concurrency: number;
}> {
  return get("/api/download/settings");
}

// ---------- Settings ----------

export function getSettings(): Promise<{
  download_dir: string | null;
  theme: string;
  asmr_one_token: string | null;
  proxy_url: string | null;
  proxy_dlsite: boolean;
  proxy_asmrone: boolean;
  asmr_one_address: string | null;
  asmr_one_username: string | null;
  asmr_one_password: string | null;
}> {
  return get("/api/settings");
}

export function setAsmrToken(token: string): Promise<void> {
  return post("/api/settings/asmr-token", { token });
}

export function clearAsmrToken(): Promise<void> {
  return del("/api/settings/asmr-token");
}

export function setSettings(
  theme?: string,
  downloadDir?: string,
  proxy?: { url?: string; dlsite?: boolean; asmrone?: boolean },
  asmr?: { address?: string; username?: string; password?: string }
): Promise<void> {
  return post("/api/settings", {
    theme,
    downloadDir,
    proxyUrl: proxy?.url,
    proxyDlsite: proxy?.dlsite,
    proxyAsmrone: proxy?.asmrone,
    asmrOneAddress: asmr?.address,
    asmrOneUsername: asmr?.username,
    asmrOnePassword: asmr?.password,
  });
}

/** 使用账号密码登录 asmr.one，成功后服务端自动保存 Token（凭据不传时使用已保存的）。 */
export function asmrLogin(
  username?: string,
  password?: string
): Promise<{ ok: boolean; message: string }> {
  return post("/api/settings/asmr-login", { username, password });
}

export function getDbPath(): Promise<string> {
  return get("/api/settings/db-path");
}

/** 导出数据库备份：服务端生成快照并以浏览器下载方式返回。 */
export function exportDatabase(): void {
  const a = document.createElement("a");
  a.href = "/api/settings/db/export";
  a.download = "dlsite_manager_backup.db";
  document.body.appendChild(a);
  a.click();
  a.remove();
}

/** 导入（还原）数据库：上传 .db 文件，覆盖服务端当前数据库。 */
export function importDatabase(file: File): Promise<string> {
  const fd = new FormData();
  fd.append("file", file);
  return request<{ message: string }>("/api/settings/db/import", {
    method: "POST",
    body: fd,
  }).then((r) => r.message);
}

export function openFolder(path: string): Promise<void> {
  return post("/api/settings/open-folder", { path });
}

// ---------- ASMR ONE 在线浏览 ----------

/** tab: popular（热门）| recommend（推荐）| all（全部，order 可选 release/dl_count/rating） */
export function asmrBrowse(
  tab: string,
  page: number,
  order?: string
): Promise<AsmrWorksPage> {
  return get(`/api/asmr/browse${qs({ tab, page, order })}`);
}

/** 账号数据：tab=favorite|playlist|history；playlist 需要时传播放列表 id */
export function asmrAccount(
  tab: string,
  page?: number,
  playlistId?: string
): Promise<AsmrWorksPage | { type: "playlists"; playlists: AsmrPlaylist[] }> {
  return get(`/api/asmr/account${qs({ tab, page, id: playlistId })}`);
}

export function asmrWorkDetail(id: number | string): Promise<AsmrOnlineWork> {
  return get(`/api/asmr/work/${id}`);
}

// ---------- DLsite 排行榜 ----------

/** term: day | week | month | year | total；limit: 20/50/100 */
export function getDlsiteRanking(term: string, limit: number): Promise<DlsiteRankingResponse> {
  return get(`/api/dlsite/ranking${qs({ term, limit })}`);
}

/** 手动触发榜单抓取；term 不传时刷新全部周期，耗时较长。 */
export function refreshDlsiteRanking(term?: string): Promise<{ ok: boolean; refreshed: string[] }> {
  return post("/api/dlsite/ranking/refresh", term ? { term } : {});
}

// ---------- 目录浏览（替代桌面端原生目录选择框） ----------

export interface DirEntry {
  name: string;
  path: string;
  isDir: boolean;
}

export interface DirListing {
  path: string;
  parent: string | null;
  entries: DirEntry[];
}

/** 浏览服务器目录；path 为空时从用户主目录开始。 */
export function listDir(path?: string): Promise<DirListing> {
  return get(`/api/fs/list${qs({ path })}`);
}

// ---------- Events ----------

/**
 * 订阅服务端下载进度（SSE）。返回取消订阅函数。
 * EventSource 断线后浏览器会自动重连。
 */
export async function onDownloadProgress(
  handler: (ev: DownloadProgressEvent) => void
): Promise<() => void> {
  const es = new EventSource("/api/events");
  es.addEventListener("download-progress", (e) => {
    if (!e.data) return;
    try {
      handler(JSON.parse(e.data) as DownloadProgressEvent);
    } catch (err) {
      console.warn("解析下载进度事件失败:", err);
    }
  });
  return () => es.close();
}
