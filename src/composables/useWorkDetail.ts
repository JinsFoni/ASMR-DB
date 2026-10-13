import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import { useRouter } from "vue-router";
import { imgUrl } from "../lib/api";
import * as api from "../lib/api";
import type { AsmrDownloadFile, AsmrOnlineWork, AsmrTreeNode, ScrapedWork, Tag, Track, Work, WorkView } from "../lib/types";
import {
  actionLabel,
  actionVariant,
  buildFakeWork,
  buildMeta,
  buildWorkView,
  resolveActions,
  resolveAgeBadge,
  resolveMode,
  resolveStatusBadge,
  tagsFromNames,
  type ActionKey,
  type DetailAction,
  type DetailMode,
  type DetailSource,
  type MetaItem,
  type StatusBadge,
} from "../lib/workDetail";
import { usePlayerStore } from "../stores/player";
import { useWorksStore } from "../stores/works";
import { useDownloadStore } from "../stores/download";
import { showToast } from "../lib/toast";

export interface UseWorkDetail {
  loading: Ref<boolean>;
  error: Ref<string>;
  mode: ComputedRef<DetailMode>;
  hasOnline: Ref<boolean>;
  /** 本地 WorkView（未入库为 null） */
  localView: Ref<WorkView | null>;
  /** 归一化后的统一视图（供 TrackList / 播放器使用） */
  view: ComputedRef<WorkView | null>;
  title: ComputedRef<string>;
  subtitle: ComputedRef<string | null>;
  rjCode: ComputedRef<string | null>;
  coverUrl: ComputedRef<string | null>;
  statusBadge: ComputedRef<StatusBadge>;
  /** 年龄指定角标文案（R-18G / R18 / 全年龄），缺失时 null */
  ageBadge: ComputedRef<string | null>;
  meta: ComputedRef<MetaItem[]>;
  actors: ComputedRef<string[]>;
  tags: ComputedRef<Tag[]>;
  editableTags: ComputedRef<boolean>;
  workId: ComputedRef<number | null>;
  description: ComputedRef<string | null>;
  descriptionNote: ComputedRef<string | null>;
  descriptionLoading: Ref<boolean>;
  actions: ComputedRef<DetailAction[]>;
  localTracks: Ref<Track[]>;
  onlineTracks: Ref<Track[]>;
  fileTree: Ref<AsmrTreeNode[] | null>;
  displayTracks: ComputedRef<Track[]>;
  tracksLoading: Ref<boolean>;
  /** asmr.one 文件树拉取中（与 tracksLoading 分离，避免拉树时隐藏音轨列表） */
  treeLoading: Ref<boolean>;
  /** 主数据不存在（如 /work/999） */
  notFound: ComputedRef<boolean>;
  copied: Ref<boolean>;
  showFileTree: Ref<boolean>;
  showFolderPicker: Ref<boolean>;
  /** DLsite 压缩包直链输入框（作品库「下载」） */
  showUrlInput: Ref<boolean>;
  dlUrl: Ref<string>;
  busyKey: Ref<ActionKey | null>;
  copyRj: () => Promise<void>;
  reload: () => Promise<void>;
  runAction: (key: ActionKey) => Promise<void>;
  onFileTreeDownload: (files: AsmrDownloadFile[]) => Promise<void>;
  onFolderPicked: (dir: string) => Promise<void>;
  onTagsUpdated: () => void;
}

export function useWorkDetail(opts: { source: DetailSource; id: Ref<string> }): UseWorkDetail {
  const router = useRouter();
  const player = usePlayerStore();
  const worksStore = useWorksStore();
  const downloadStore = useDownloadStore();

  const loading = ref(true);
  const error = ref("");
  const copied = ref(false);

  // 主数据（按 source 三选一）
  const localView = ref<WorkView | null>(null);
  const asmrWork = ref<AsmrOnlineWork | null>(null);
  const scrapedWork = ref<ScrapedWork | null>(null);
  const dlsiteDescription = ref("");
  const descriptionLoading = ref(false);
  /** asmr.one 无评分时从 DLsite 作品信息接口后台补齐的评分 */
  const dlsiteRating = ref<number | null>(null);

  const hasOnline = ref(false);
  const localTracks = ref<Track[]>([]);
  const onlineTracks = ref<Track[]>([]);
  const fileTree = ref<AsmrTreeNode[] | null>(null);
  const tracksLoading = ref(false);
  const treeLoading = ref(false);
  const busyKey = ref<ActionKey | null>(null);

  /** 加载代次：详情→详情 快速跳转时，丢弃上一轮迟到的后台请求结果 */
  let loadSeq = 0;

  const showFileTree = ref(false);
  const showFolderPicker = ref(false);
  const showUrlInput = ref(false);
  const dlUrl = ref("");

  const rjCode = computed<string | null>(
    () => localView.value?.work.rj_code ?? asmrWork.value?.rj_code ?? scrapedWork.value?.rj_code ?? null
  );

  const hasLocalWork = computed(() => localView.value != null);

  /** spec §2.2 模式判定 */
  const mode = computed<DetailMode>(() =>
    resolveMode({
      source: opts.source,
      hasOnline: hasOnline.value,
      localTrackCount: localTracks.value.length,
    })
  );

  const rawWork = computed<Work | null>(() => {
    if (localView.value) return localView.value.work;
    if (asmrWork.value || scrapedWork.value) {
      return buildFakeWork(opts.source, opts.id.value, asmrWork.value, scrapedWork.value);
    }
    return null;
  });

  const title = computed(
    () =>
      localView.value?.work.title_zh ||
      localView.value?.work.title_ja ||
      // 在线页元数据统一从 DLsite 获取：DLsite 标题优先（asmr.one 标题带 RJ 前缀噪音）
      scrapedWork.value?.title_zh ||
      scrapedWork.value?.title_ja ||
      asmrWork.value?.title ||
      rjCode.value ||
      `#${opts.id.value}`
  );

  const subtitle = computed<string | null>(() => {
    // 已翻译（有中文标题）时不展示日文原标题
    if (rawWork.value?.title_zh) return null;
    const ja = rawWork.value?.title_ja ?? null;
    return ja && ja !== title.value ? ja : null;
  });

  const coverUrl = computed<string | null>(() => {
    const u = rawWork.value?.cover_url ?? null;
    return u ? imgUrl(u) : null;
  });

  // 仅非作品库页的已入库作品，角标才可点击跳本地详情（作品库页自身即本地详情）
  const statusBadge = computed<StatusBadge>(() =>
    resolveStatusBadge(
      localView.value?.download_status ?? "not_downloaded",
      hasLocalWork.value,
      opts.source !== "work" && hasLocalWork.value
    )
  );

  /** 年龄指定角标：入库标识之后展示（R-18G / R18 / 全年龄） */
  const ageBadge = computed<string | null>(() => resolveAgeBadge(rawWork.value?.age_class));

  const meta = computed<MetaItem[]>(() => {
    const w = rawWork.value;
    if (!w) return [];
    // 本地/来源无评分时，用后台从 DLsite 补齐的评分兜底
    if (w.rating == null && dlsiteRating.value != null) {
      return buildMeta(opts.source, { ...w, rating: dlsiteRating.value });
    }
    return buildMeta(opts.source, w);
  });

  const actors = computed<string[]>(
    () =>
      localView.value?.actors ??
      // 统一元数据：DLsite 声优优先，asmr.one 兜底
      (scrapedWork.value?.actors.length ? scrapedWork.value.actors : asmrWork.value?.vas) ??
      []
  );

  /**
   * 标签：已入库时用本地标签（作品库单一来源，含用户编辑）；
   * 未入库时用来源自身的标签名（asmr.one / DLsite），转为只读 Tag 形态。
   */
  const tags = computed<Tag[]>(() => {
    if (localView.value) return localView.value.tags;
    if (opts.source === "asmr") {
      // 统一元数据：DLsite 标签优先，asmr.one 兜底
      const names = scrapedWork.value?.tags.length
        ? scrapedWork.value.tags
        : asmrWork.value?.tags ?? [];
      return tagsFromNames(names);
    }
    if (opts.source === "dlsite") return tagsFromNames(scrapedWork.value?.tags ?? []);
    return [];
  });

  const editableTags = computed(() => opts.source === "work" && localView.value != null);

  const workId = computed<number | null>(() => localView.value?.work.id ?? null);

  const description = computed<string | null>(() => {
    if (opts.source === "work") {
      return localView.value?.work.description_zh ?? localView.value?.work.description ?? null;
    }
    // 在线页统一元数据：DLsite 简介优先，asmr.one 兜底
    return scrapedWork.value?.description ?? asmrWork.value?.description ?? dlsiteDescription.value ?? null;
  });

  const descriptionNote = computed<string | null>(() => {
    if (opts.source === "work") {
      return localView.value?.work.description_zh ? "中文翻译" : null;
    }
    if (opts.source === "asmr") {
      // 简介既非 DLsite 统一抓取、也非 asmr.one 自带时，才标注来源
      return !scrapedWork.value?.description && !asmrWork.value?.description && dlsiteDescription.value
        ? "（来自 DLsite）"
        : null;
    }
    return null;
  });

  /** spec §8：local → 本地音轨；asmr → 在线音轨；dlsite → 空 */
  const displayTracks = computed<Track[]>(() =>
    mode.value === "local" ? localTracks.value : mode.value === "asmr" ? onlineTracks.value : []
  );

  /** spec §8：asmr 模式用在线音轨数与总时长，但保留真实 work.id 供高亮匹配 */
  const view = computed<WorkView | null>(() => {
    // DLsite 页在 dlsite 模式下不渲染音轨卡片（保持该页现状）；作品库则显示「暂无音轨」
    if (opts.source === "dlsite" && mode.value === "dlsite") return null;
    if (opts.source === "work" && localView.value) {
      if (mode.value === "local") return localView.value;
      return {
        ...localView.value,
        track_count: onlineTracks.value.length,
        total_duration_sec: onlineTracks.value.reduce((s, t) => s + (t.duration_sec || 0), 0),
      };
    }
    const w = rawWork.value;
    if (!w) return null;
    return buildWorkView(
      w,
      tags.value,
      actors.value,
      localView.value,
      displayTracks.value.length,
      displayTracks.value.reduce((s, t) => s + (t.duration_sec || 0), 0)
    );
  });

  const actions = computed<DetailAction[]>(() => {
    const keys = resolveActions({
      source: opts.source,
      mode: mode.value,
      hasOnline: hasOnline.value,
      hasLocalWork: hasLocalWork.value,
      hasRj: rjCode.value != null,
    });
    return keys.map((key, i) => ({
      key,
      label: actionLabel(key, opts.source),
      variant: actionVariant(key, i),
    }));
  });

  const notFound = computed(
    () =>
      !loading.value &&
      !error.value &&
      localView.value == null &&
      asmrWork.value == null &&
      scrapedWork.value == null
  );

  /**
   * 拉取 asmr.one 在线音轨。spec §2.3：始终探测（不拉文件树）。
   * 未配置 token / asmr.one 无此作品 / 报错 → 返回 null（静默降级）。
   */
  async function fetchOnlineTracks(rj: string): Promise<Track[] | null> {
    try {
      const s = await api.getSettings();
      if (!s.asmr_one_token) return null;
    } catch {
      return null;
    }
    try {
      return await api.listAsmroneTracks(rj);
    } catch {
      return null;
    }
  }

  async function loadLocalState() {
    const rj = rjCode.value;
    if (!rj) {
      localView.value = null;
      return;
    }
    try {
      localView.value = await api.getWorkByRj(rj);
    } catch {
      localView.value = null;
    }
  }

  async function load() {
    const seq = ++loadSeq;
    loading.value = true;
    error.value = "";
    copied.value = false;
    localView.value = null;
    asmrWork.value = null;
    scrapedWork.value = null;
    dlsiteDescription.value = "";
    dlsiteRating.value = null;
    localTracks.value = [];
    onlineTracks.value = [];
    fileTree.value = null;
    // hasOnline 必须重置：详情→详情跳转时若残留 true，mode 会在探测完成前被误判为 asmr
    hasOnline.value = false;
    tracksLoading.value = false;
    treeLoading.value = false;
    showUrlInput.value = false;
    dlUrl.value = "";

    const id = opts.id.value;
    try {
      // ── 首屏关键路径：只等主数据（本地 DB 查询 / 单次详情请求）──
      if (opts.source === "work") {
        const v = await api.getWork(Number(id));
        if (seq !== loadSeq) return;
        if (!v) {
          localView.value = null;
          return;
        }
        localView.value = v;
        localTracks.value = await api.listTracks(v.work.id);
        if (seq !== loadSeq) return;
      } else if (opts.source === "asmr") {
        asmrWork.value = await api.asmrWorkDetail(id);
        if (seq !== loadSeq) return;
        await loadLocalState();
        if (seq !== loadSeq) return;
      } else {
        scrapedWork.value = await api.previewByRj(id);
        if (seq !== loadSeq) return;
        await loadLocalState();
        if (seq !== loadSeq) return;
      }
    } catch (e) {
      if (seq !== loadSeq) return;
      error.value = String(e);
    } finally {
      // 只有当前代次才有权结束 loading（快速跳转时由新一轮接管）
      if (seq === loadSeq) loading.value = false;
    }

    if (seq !== loadSeq || error.value) return;
    // ── 次要数据后台补齐：绝不阻塞标题/封面/按钮的首屏渲染 ──
    if (opts.source === "asmr") {
      loadDlsiteDescription(seq);
      // 元数据统一从 DLsite 获取：asmr.one 首屏先渲染，DLsite 字段后台合并
      loadDlsiteMeta(seq);
    }
    loadSecondary(seq).catch(() => {});
  }

  /**
   * 在线音轨 / 文件树等次要数据，走外网（0.5–1.2s），必须在首屏渲染后后台补齐。
   * 此前这些请求串行 await 在 loading 门控内，导致整页（含标题）一起等待，体感明显变慢。
   */
  async function loadSecondary(seq: number) {
    const rj = rjCode.value;
    if (!rj) return;
    // 作品无评分时，用 RJ 号从 DLsite 作品信息接口后台补齐（不阻塞首屏）
    if (rawWork.value?.rating == null) {
      api
        .getDlsiteRating(rj)
        .then((r) => {
          if (seq === loadSeq) dlsiteRating.value = r;
        })
        .catch(() => {});
    }
    // 作品库本地已有音轨：音轨区立即可用，无需 loading 占位
    // （但仍需探测在线音轨，用于「在线播放 / ASMR One下载」按钮显隐）
    const localReady = opts.source === "work" && localTracks.value.length > 0;
    if (!localReady) tracksLoading.value = true;
    try {
      // asmr / DLsite 来源：音轨与文件树无依赖，并行拉取（与改动前两页一致）
      if (opts.source !== "work") {
        treeLoading.value = true;
        const [t, tree] = await Promise.all([
          fetchOnlineTracks(rj),
          api.listAsmroneTree(rj).catch(() => null),
        ]);
        if (seq !== loadSeq) return;
        onlineTracks.value = t ?? [];
        hasOnline.value = (t?.length ?? 0) > 0;
        fileTree.value = tree;
        return;
      }
      // 作品库：先探测在线音轨（mode 依赖 hasOnline），再按 mode 决定是否拉文件树
      const t = await fetchOnlineTracks(rj);
      if (seq !== loadSeq) return;
      onlineTracks.value = t ?? [];
      hasOnline.value = (t?.length ?? 0) > 0;
      // 文件树仅 asmr 模式需要（spec §8 音频/全部文件标签页）；
      // 本地有音轨时 mode === 'local'，按 spec §2.3 不拉树。
      if (mode.value === "asmr") {
        treeLoading.value = true;
        const tree = await api.listAsmroneTree(rj).catch(() => null);
        if (seq === loadSeq) fileTree.value = tree;
      }
    } finally {
      if (seq === loadSeq) {
        tracksLoading.value = false;
        treeLoading.value = false;
      }
    }
  }

  /**
   * ASMR ONE 页统一元数据：后台从 DLsite 抓取（与 DLsite 详情页共用同一份
   * 服务端短 TTL 缓存），写入 scrapedWork 供 buildFakeWork 以 DLsite 字段优先合并。
   * asmr.one 数据兜底并保留在线特有字段（时长、音轨）。
   */
  async function loadDlsiteMeta(seq: number) {
    const rj = asmrWork.value?.rj_code;
    if (!rj) return;
    try {
      const meta = await api.previewByRj(rj, "dlsite");
      if (seq !== loadSeq) return;
      scrapedWork.value = meta;
    } catch {
      // DLsite 抓取失败时保持 asmr.one 自身数据展示，静默降级
    }
  }

  /** ASMR ONE 页补充 DLsite 简介（asmr.one API 不提供简介） */
  async function loadDlsiteDescription(seq: number) {
    const rj = asmrWork.value?.rj_code;
    if (!rj) return;
    descriptionLoading.value = true;
    try {
      const d = await api.getDlsiteDescription(rj);
      // 代次守卫：详情→详情快速跳转时丢弃上一轮迟到的响应，避免 A 的简介写入 B 的页面
      if (seq !== loadSeq) return;
      dlsiteDescription.value = d;
    } catch {
      if (seq === loadSeq) dlsiteDescription.value = "";
    } finally {
      if (seq === loadSeq) descriptionLoading.value = false;
    }
  }

  function onTagsUpdated() {
    load();
  }

  /** 复制 RJ 号到剪贴板；成功后短暂显示「已复制」。 */
  async function copyRj() {
    const rj = rjCode.value;
    if (!rj) return;
    try {
      await navigator.clipboard.writeText(rj);
    } catch {
      // 剪贴板 API 被拒（非安全上下文等）时退回 execCommand
      const ta = document.createElement("textarea");
      ta.value = rj;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      ta.remove();
    }
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  }

  async function playAll() {
    const w = view.value;
    if (!w || !localTracks.value.length) return;
    const progress = await api.getPlayProgress(w.work.id);
    let startIndex = 0;
    let initialTime = 0;
    if (progress) {
      const idx = localTracks.value.findIndex((t) => t.id === progress.track_id);
      if (idx >= 0) {
        startIndex = idx;
        initialTime = progress.last_position || 0;
      }
    }
    player.playFromWork(w, localTracks.value, startIndex, initialTime);
    router.push({ name: "player" });
  }

  async function playOnline() {
    const w = view.value;
    if (!w) return;
    if (!onlineTracks.value.length) {
      alert("asmr.one 未找到音频文件");
      return;
    }
    player.playFromWork(w, onlineTracks.value, 0, 0);
    router.push({ name: "player" });
  }

  /** 入库后跳转本地详情；用 replace 替换历史记录，浏览器返回直接回到浏览页（不经过在线详情中间页） */
  async function importWork() {
    const rj = rjCode.value;
    if (!rj) return;
    const res = await api.importByRj(rj);
    worksStore.fetchWorks();
    worksStore.loadTags();
    worksStore.markImported(rj, res.work.work.id);
    router.replace(`/work/${res.work.work.id}`);
  }

  function goLocal() {
    if (localView.value) router.push(`/work/${localView.value.work.id}`);
  }

  /** spec §6 下载流程：静默入库 → 拉文件树 → 弹选择框 → 下载 → 留在当前页 */
  async function openAsmrDownload() {
    const rj = rjCode.value;
    if (!rj) return;
    const s = await api.getSettings();
    if (!s.asmr_one_token) {
      alert("请先在「设置 -> asmr.one」登录获取 Token 后再试");
      return;
    }
    if (!localView.value) {
      const res = await api.importByRj(rj);
      worksStore.fetchWorks();
      worksStore.loadTags();
      worksStore.markImported(rj, res.work.work.id);
      localView.value = res.work;
    }
    showFileTree.value = true;
    // asmr 模式下文件树已在 load() 中拉好，避免重复请求
    if (fileTree.value) return;
    treeLoading.value = true;
    try {
      fileTree.value = await api.listAsmroneTree(rj);
    } catch (e) {
      alert(`获取 asmr.one 文件树失败: ${e}`);
      showFileTree.value = false;
    } finally {
      treeLoading.value = false;
    }
  }

  async function onFileTreeDownload(files: AsmrDownloadFile[]) {
    showFileTree.value = false;
    const id = localView.value?.work.id;
    if (!id) return;
    try {
      await api.downloadAsmroneFiles(id, files);
      alert(`已开始从 asmr.one 下载 ${files.length} 个文件，可在「下载管理」查看实时进度。`);
      await loadLocalState();
    } catch (e) {
      alert(`下载失败: ${e}`);
    }
  }

  function openDlsite() {
    const url =
      rawWork.value?.dlsite_url ??
      (rjCode.value
        ? `https://www.dlsite.com/maniax/work/=/product_id/${rjCode.value}.html`
        : null);
    if (url) window.open(url, "_blank", "noopener");
  }

  function openAsmrOne() {
    const rj = rjCode.value;
    if (rj) window.open(`https://www.asmr.one/work/${rj}`, "_blank", "noopener");
  }

  /** 作品库「下载」：首次点击展开内联直链输入框，填好后再点才真正开始下载 */
  async function startDlDownload() {
    const id = localView.value?.work.id;
    if (!id) return;
    if (!dlUrl.value.trim()) {
      showUrlInput.value = true;
      return;
    }
    await downloadStore.startDownload(id, dlUrl.value.trim());
    showUrlInput.value = false;
  }

  async function scanTracks() {
    const id = localView.value?.work.id;
    if (!id) return;
    const res = await api.scanAudioTracks(id);
    if (!res.ok) alert(res.message || "扫描失败");
    else await load();
  }

  async function translateWork() {
    const id = localView.value?.work.id;
    if (!id) return;
    const res = await api.enqueueTranslation(id);
    showToast(res.message || "已加入翻译队列", "success");
  }

  async function deleteWork() {
    const id = localView.value?.work.id;
    if (!id) return;
    if (!confirm(`确定删除「${title.value}」？将从数据库移除该作品。`)) return;
    await api.deleteWork(id);
    worksStore.removeWork(id);
    router.push({ name: "home" });
  }

  async function onFolderPicked(dir: string) {
    showFolderPicker.value = false;
    const rj = rjCode.value;
    if (!rj) return;
    try {
      const res = await api.bindLocalFolder(rj, dir);
      alert(
        `关联成功！扫描到 ${res.trackCount} 首音轨${res.groupName ? `，已自动分配分组「${res.groupName}」` : ""}`
      );
      await load();
      worksStore.fetchWorks();
      worksStore.loadGroups();
    } catch (e) {
      alert(`关联失败: ${e}`);
    }
  }

  /** 动作分发：统一处理 loading 状态与错误提示 */
  async function runAction(key: ActionKey) {
    if (busyKey.value) return;
    busyKey.value = key;
    try {
      switch (key) {
        case "play-all":
          await playAll();
          break;
        case "online-play":
          await playOnline();
          break;
        case "import":
          await importWork();
          break;
        case "go-local":
          goLocal();
          break;
        case "asmr-download":
          await openAsmrDownload();
          break;
        case "dl-download":
          await startDlDownload();
          break;
        case "scan-tracks":
          await scanTracks();
          break;
        case "bind-local":
          showFolderPicker.value = true;
          break;
        case "translate":
          await translateWork();
          break;
        case "delete":
          await deleteWork();
          break;
        case "open-dlsite":
          openDlsite();
          break;
        case "open-asmr":
          openAsmrOne();
          break;
      }
    } catch (e) {
      alert(`操作失败: ${e}`);
    } finally {
      busyKey.value = null;
    }
  }

  watch(opts.id, (v, old) => {
    if (v && v !== old) load();
  });
  load();

  return {
    loading,
    error,
    mode,
    hasOnline,
    localView,
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
    localTracks,
    onlineTracks,
    fileTree,
    displayTracks,
    tracksLoading,
    treeLoading,
    notFound,
    copied,
    showFileTree,
    showFolderPicker,
    showUrlInput,
    dlUrl,
    busyKey,
    copyRj,
    reload: load,
    runAction,
    onFileTreeDownload,
    onFolderPicked,
    onTagsUpdated,
  };
}