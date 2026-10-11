/**
 * 作品详情页的纯逻辑：模式判定、按钮矩阵、元数据行、状态角标、三种来源归一化。
 *
 * 本文件被 `node --test` 直接加载（Node 原生类型剥离），因此：
 * 除了下面这一行 `import type`，不得出现任何相对「值」导入
 * （Node 不做扩展名解析，省略 .ts 的相对值导入会运行时报错）。
 */
import type { Work, WorkView, Tag, AsmrOnlineWork, ScrapedWork } from "./types";

export type DetailSource = "work" | "asmr" | "dlsite";
export type DetailMode = "local" | "asmr" | "dlsite";

export type ActionKey =
  | "play-all"
  | "online-play"
  | "import"
  | "go-local"
  | "asmr-download"
  | "dl-download"
  | "scan-tracks"
  | "bind-local"
  | "translate"
  | "delete"
  | "open-dlsite"
  | "open-asmr";

export type ActionVariant = "primary" | "outline" | "ghost";

export interface MetaItem {
  icon: string;
  text: string;
}

export interface StatusBadge {
  text: string;
  tone: "green" | "accent" | "neutral" | "yellow";
  clickable: boolean;
}

export interface DetailAction {
  key: ActionKey;
  label: string;
  variant: ActionVariant;
}

export interface ModeInput {
  source: DetailSource;
  hasOnline: boolean;
  localTrackCount: number;
}

export interface ActionInput {
  source: DetailSource;
  mode: DetailMode;
  hasOnline: boolean;
  hasLocalWork: boolean;
  hasRj: boolean;
}

/** spec §2.2 模式判定公式 */
export function resolveMode(input: ModeInput): DetailMode {
  const { source, hasOnline, localTrackCount } = input;
  if (source === "asmr") return "asmr";
  if (source === "work" && localTrackCount > 0) return "local";
  return hasOnline ? "asmr" : "dlsite";
}

/** spec §4 按钮矩阵，返回按钮键的有序列表 */
export function resolveActions(input: ActionInput): ActionKey[] {
  const { source, mode, hasOnline, hasLocalWork, hasRj } = input;
  const keys: ActionKey[] = [];

  if (source === "work") {
    // 作品库：本地操作在前，在线操作在后（场景 4/4b 顺序）
    if (mode === "local") keys.push("play-all");
    if (mode === "local") keys.push("dl-download");
    if (mode !== "local") {
      if (hasOnline) keys.push("online-play", "asmr-download");
      keys.push("dl-download");
    }
    keys.push("scan-tracks", "bind-local", "translate");
    if (mode === "local" && hasOnline) keys.push("online-play", "asmr-download");
  } else {
    // ASMR ONE / DLsite：先在线播放，再入库或本地详情，再 asmr.one 下载
    if (hasOnline) keys.push("online-play");
    if (hasLocalWork) keys.push("go-local");
    else if (hasRj) keys.push("import");
    if (hasOnline) keys.push("asmr-download");
  }

  if (hasRj) keys.push("open-dlsite");
  // spec §4.1：仅 DLsite 页的 ASMR One 外链受 hasOnline 约束
  if (source === "dlsite" ? hasOnline : true) keys.push("open-asmr");

  if (source === "work") keys.push("delete");

  return keys;
}

/** 按钮文案（spec §4.2） */
export function actionLabel(key: ActionKey, source: DetailSource): string {
  switch (key) {
    case "play-all":
      return "播放";
    case "online-play":
      return "在线播放";
    case "import":
      return "入库";
    case "go-local":
      return "本地详情";
    case "asmr-download":
      return source === "work" ? "ASMR One下载" : "下载";
    case "dl-download":
      return "下载";
    case "scan-tracks":
      return "扫描音轨";
    case "bind-local":
      return "关联本地";
    case "translate":
      return "LLM 翻译";
    case "delete":
      return "删除";
    case "open-dlsite":
      return "DLsite";
    case "open-asmr":
      return "ASMR One";
  }
}

/** 按钮样式：首个为 primary；外链/关联本地/删除为 ghost；其余 outline */
export function actionVariant(key: ActionKey, index: number): ActionVariant {
  if (index === 0) return "primary";
  if (key === "open-dlsite" || key === "open-asmr" || key === "bind-local" || key === "delete") {
    return "ghost";
  }
  return "outline";
}

/**
 * 元数据方格（详情页两列布局，按行优先顺序）：
 * 第一行 售出/收藏，第二行 发售/评分，第三行 社团/声优（声优由详情页追加）。
 * 旧字段（价格/时长/容量）不再进方格。
 */
export function buildMeta(_source: DetailSource, w: Work): MetaItem[] {
  const items: MetaItem[] = [];
  if (w.dl_count != null) items.push({ icon: "📦", text: `售出：${w.dl_count.toLocaleString()}` });
  if (w.wishlist_count != null) items.push({ icon: "❤️", text: `收藏：${w.wishlist_count.toLocaleString()}` });
  if (w.sale_date) items.push({ icon: "📅", text: `发售：${String(w.sale_date).slice(0, 10)}` });
  if (w.rating != null && w.rating > 0) {
    // 直接输出原值（如 4.73），不用 toFixed 避免 4.84 → 4.8 的精度丢失
    items.push({ icon: "⭐", text: `评分：${w.rating}` });
  }
  if (w.circle_name) items.push({ icon: "🏢", text: `社团：${w.circle_name}` });
  return items;
}

/**
 * 状态角标（spec §7）。
 * 已入库 → 显示本地下载状态；未入库 → 一律「未入库」（黄）。
 * `clickable` 由调用方给出（仅「非作品库页 + 已入库」为真），未入库恒不可点击。
 */
export function resolveStatusBadge(
  downloadStatus: string,
  hasLocalWork: boolean,
  clickable: boolean
): StatusBadge {
  if (!hasLocalWork) {
    return { text: "未入库", tone: "yellow", clickable: false };
  }
  const canClick = clickable;
  if (downloadStatus === "downloaded") {
    return { text: "已下载", tone: "green", clickable: canClick };
  }
  if (downloadStatus === "downloading") {
    return { text: "下载中", tone: "accent", clickable: canClick };
  }
  return { text: "未下载", tone: "neutral", clickable: canClick };
}

/**
 * 年龄指定角标（详情页 RJ 号行，紧跟入库标识之后）。
 * r18g → R-18G，r18 → R18，general → 全年龄，未知/缺失 → 不显示。
 */
export function resolveAgeBadge(ageClass: string | null | undefined): string | null {
  switch (ageClass) {
    case "r18g":
      return "R-18G";
    case "r18":
      return "R18";
    case "general":
      return "全年龄";
    default:
      return null;
  }
}

/**
 * 归一化：本地 WorkView 优先；本地缺失时用在线/抓取数据构造临时 WorkView。
 * 这是原先 ASMR ONE 与 DLsite 两处重复 `fakeView` 的唯一实现处。
 */
export function buildWorkView(
  work: Work,
  tags: Tag[],
  actors: string[],
  local: WorkView | null,
  trackCount: number,
  totalDurationSec: number
): WorkView {
  return {
    work,
    tags: local ? local.tags : tags,
    actors: local ? local.actors : actors,
    download_status: local ? local.download_status : "not_downloaded",
    local_path: local ? local.local_path : null,
    track_count: trackCount,
    total_duration_sec: totalDurationSec,
    group: local ? local.group : null,
  };
}

/**
 * 把 ASMR ONE / DLsite 的标签名数组转成只读 Tag 形态，供统一展示组件渲染。
 * 负数 id 表示「非本地标签」，仅用于 v-for key，不会被提交回后端。
 */
export function tagsFromNames(names: string[]): Tag[] {
  return names.map((name, i) => ({
    id: -(i + 1),
    name,
    color: "#888888",
    is_preset: false,
  }));
}

/**
 * 由 ASMR ONE / DLsite 数据构造临时 Work（id 为 0 表示未入库）。
 * 元数据统一以 DLsite 抓取结果（s）优先；asmr.one（a）只兜底
 * 并提供在线特有字段（时长、日文原标题）。
 */
export function buildFakeWork(
  source: DetailSource,
  id: string | number,
  a: AsmrOnlineWork | null,
  s: ScrapedWork | null
): Work {
  const rj = a?.rj_code ?? s?.rj_code ?? String(id);
  return {
    id: 0,
    rj_code: rj,
    title_ja: s?.title_ja ?? a?.title ?? null,
    title_zh: s?.title_zh ?? null,
    title_en: s?.title_en ?? null,
    circle_name: s?.circle_name ?? a?.circle_name ?? null,
    circle_id: s?.circle_id ?? null,
    cover_url: s?.cover_url ?? a?.cover_url ?? null,
    work_type: s?.work_type ?? "voice",
    dl_count: s?.dl_count ?? a?.dl_count ?? null,
    wishlist_count: s?.wishlist_count ?? null,
    rating: s?.rating ?? a?.rating ?? null,
    sale_date: s?.sale_date ?? a?.release ?? null,
    description: s?.description ?? a?.description ?? null,
    description_zh: source === "asmr" ? (a?.description ?? null) : null,
    age_class: s?.age_class ?? a?.age_class ?? null,
    duration_min: a?.duration_sec ? Math.round(a.duration_sec / 60) : (s?.duration_min ?? null),
    file_size_mb: s?.file_size_mb ?? null,
    dlsite_url: s?.dlsite_url ?? `https://www.dlsite.com/maniax/work/=/product_id/${rj}.html`,
    created_at: null,
    updated_at: null,
  };
}