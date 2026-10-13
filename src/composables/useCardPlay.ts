import { ref } from "vue";
import * as api from "../lib/api";
import type { AsmrOnlineWork, WorkView } from "../lib/types";
import { buildFakeWork, buildWorkView, tagsFromNames } from "../lib/workDetail";
import { usePlayerStore } from "../stores/player";
import { useWorksStore } from "../stores/works";

/**
 * 卡片左下角播放按钮的共用逻辑：
 * 已入库（本地有音轨）优先播放本地文件，未入库的在线作品按 RJ 号走 asmr.one
 * 在线播放，统一交给全局播放器（MiniPlayer 自动出现，页面不跳转）。
 */
export function useCardPlay() {
  const player = usePlayerStore();
  const worksStore = useWorksStore();
  /** 正在拉取音轨的卡片 key（本地作品 id / 在线卡片 id） */
  const playingId = ref<string | number | null>(null);

  /** 播放本地作品：恢复上次进度后交给全局播放器。key 为卡片侧的 loading 标识。 */
  async function playView(view: WorkView, key: number = view.work.id) {
    if (player.work?.work.id === view.work.id) {
      player.togglePlay();
      return;
    }
    playingId.value = key;
    try {
      const tracks = await api.listTracks(view.work.id);
      if (!tracks.length) {
        alert("本地未找到音频文件");
        return;
      }
      let startIndex = 0;
      let initialTime = 0;
      const progress = await api.getPlayProgress(view.work.id).catch(() => null);
      if (progress) {
        const idx = tracks.findIndex((t) => t.id === progress.track_id);
        if (idx >= 0) {
          startIndex = idx;
          initialTime = progress.last_position || 0;
        }
      }
      player.playFromWork(view, tracks, startIndex, initialTime);
    } finally {
      playingId.value = null;
    }
  }

  /** 在线页/最新页卡片：本地有音轨走本地，否则按 RJ 号从 asmr.one 在线播放。 */
  async function playItem(item: AsmrOnlineWork) {
    if (isItemCurrent(item)) {
      player.togglePlay();
      return;
    }
    const key = item.local_work_id ?? item.id;
    const localId = item.local_work_id ?? null;
    if (localId) {
      const view =
        worksStore.works.find((v) => v.work.id === localId) ??
        (await api.getWork(localId).catch(() => null));
      // 本地确实有音轨才走本地播放；本地无文件回退在线
      if (view && view.track_count > 0) {
        await playView(view, key);
        return;
      }
    }
    if (!item.rj_code) {
      alert("该作品缺少 RJ 号，无法在线播放");
      return;
    }
    playingId.value = key;
    try {
      const tracks = await api.listAsmroneTracks(item.rj_code);
      if (!tracks.length) {
        alert("asmr.one 未找到音频文件");
        return;
      }
      const view = buildWorkView(
        buildFakeWork("asmr", item.id, item, null),
        tagsFromNames(item.tags),
        item.vas,
        null,
        tracks.length,
        item.duration_sec ?? 0
      );
      player.playFromWork(view, tracks, 0, 0);
    } catch (e) {
      alert(`在线播放失败: ${e}`);
    } finally {
      playingId.value = null;
    }
  }

  /** 该卡片的音频是否已加载进播放器（图标切暂停，点击变暂停/继续） */
  function isItemCurrent(item: AsmrOnlineWork): boolean {
    const cur = player.work;
    if (!cur) return false;
    if (item.local_work_id != null) return cur.work.id === item.local_work_id;
    return (
      !!item.rj_code &&
      !!cur.work.rj_code &&
      cur.work.rj_code.toUpperCase() === item.rj_code.toUpperCase()
    );
  }

  /** 该卡片的播放按钮是否正在拉取音轨 */
  function isItemLoading(item: AsmrOnlineWork): boolean {
    return playingId.value === (item.local_work_id ?? item.id);
  }

  /** 本地作品卡片：该作品是否已加载进播放器 */
  function isViewCurrent(view: WorkView): boolean {
    return player.work?.work.id === view.work.id;
  }

  /** 本地作品卡片：播放按钮是否正在拉取音轨 */
  function isViewLoading(view: WorkView): boolean {
    return playingId.value === view.work.id;
  }

  return { playingId, playView, playItem, isItemCurrent, isItemLoading, isViewCurrent, isViewLoading };
}
