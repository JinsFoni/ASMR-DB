// TypeScript 类型定义，与 Rust models 对应

export interface Work {
  id: number;
  rj_code: string;
  title_ja: string | null;
  title_zh: string | null;
  title_en: string | null;
  circle_name: string | null;
  circle_id: string | null;
  cover_url: string | null;
  work_type: string;
  price: number | null;
  sale_date: string | null;
  description: string | null;
  age_class: string | null;
  duration_min: number | null;
  file_size_mb: number | null;
  dlsite_url: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface WorkView {
  work: Work;
  tags: Tag[];
  actors: string[];
  download_status: string;
  local_path: string | null;
  track_count: number;
  total_duration_sec: number;
  group: Group | null;
}

export interface Tag {
  id: number;
  name: string;
  color: string;
  is_preset: boolean;
}

export interface Group {
  id: number;
  name: string;
  color: string;
}

export interface SubtitleInfo {
  title: string;
  extension: string;
  url: string;
}

export interface Track {
  id: number;
  work_id: number;
  file_path: string;
  track_number: number | null;
  title: string | null;
  duration_sec: number | null;
  file_format: string | null;
  file_size: number | null;
  subtitles?: SubtitleInfo[];
}

export interface PlayHistory {
  id: number;
  work_id: number;
  track_id: number | null;
  last_position: number;
  play_count: number;
  last_played_at: string | null;
}

export interface ScrapedWork {
  rj_code: string;
  title_ja: string | null;
  title_zh: string | null;
  title_en: string | null;
  circle_name: string | null;
  circle_id: string | null;
  cover_url: string | null;
  work_type: string;
  price: number | null;
  sale_date: string | null;
  description: string | null;
  age_class: string | null;
  duration_min: number | null;
  file_size_mb: number | null;
  dlsite_url: string | null;
  actors: string[];
  tags: string[];
}

export interface DownloadTask {
  rj_code: string;
  title: string;
  status: "queued" | "downloading" | "paused" | "done" | "error" | "cancelled";
  progress: number;
  speed: number;
  bytes_done: number;
  bytes_total: number;
  error: string | null;
}

export interface DownloadProgressEvent {
  rjCode: string;
  status: string;
  progress: number;
  speed: number;
  bytesDone: number;
  bytesTotal: number;
  error: string | null;
}

export interface ScannedItem {
  rjCode: string;
  path: string;
  found: boolean;
  title: string | null;
  groupName: string | null;
}

export interface WorkListResponse {
  items: WorkView[];
  total: number;
}

export interface AsmrTreeNode {
  title: string;
  nodeType: string; // "folder" | "audio" | "text" | "image" | "other"
  extension: string;
  hash: string;
  downloadUrl: string;
  size: number | null;
  duration: number | null;
  children: AsmrTreeNode[];
}

export interface AsmrDownloadFile {
  title: string;
  downloadUrl: string;
  relativePath: string;
}
