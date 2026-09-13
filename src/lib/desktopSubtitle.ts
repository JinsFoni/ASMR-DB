// 悬浮歌词窗口管理（浏览器弹窗版）。
// 浏览器无法做到置顶/透明无边框，改用 window.open 弹出小窗，
// 主窗口与弹窗之间用 BroadcastChannel 同步字幕文本与关闭事件。

const CHANNEL_NAME = "dlsite-asmr-subtitle";
const WINDOW_NAME = "dlsite-asmr-desktop-subtitle";
const WIDTH = 560;
const HEIGHT = 150;

let channel: BroadcastChannel | null = null;
let win: Window | null = null;

function getChannel(): BroadcastChannel {
  if (!channel) channel = new BroadcastChannel(CHANNEL_NAME);
  return channel;
}

/** 打开（或聚焦）悬浮歌词弹窗，屏幕底部居中。 */
export function openDesktopSubtitleWindow(): void {
  if (win && !win.closed) {
    win.focus();
    return;
  }
  const left = Math.max(0, Math.round((window.screen.availWidth - WIDTH) / 2));
  const top = Math.max(0, window.screen.availHeight - HEIGHT - 60);
  const url = `${location.origin}/#/desktop-subtitle`;
  win = window.open(
    url,
    WINDOW_NAME,
    `popup=yes,width=${WIDTH},height=${HEIGHT},left=${left},top=${top}`
  );
}

/** 关闭悬浮歌词弹窗。 */
export function closeDesktopSubtitleWindow(): void {
  try {
    win?.close();
  } catch {
    // ignore
  }
  win = null;
}

/** 向悬浮歌词窗口推送当前字幕文本。 */
export function pushDesktopSubtitleText(text: string): void {
  try {
    getChannel().postMessage({ type: "text", text });
  } catch {
    // ignore
  }
}

/** 悬浮歌词窗口自身关闭时通知主窗口（供弹窗页面调用）。 */
export function emitSubtitleClosed(): void {
  try {
    getChannel().postMessage({ type: "close" });
  } catch {
    // ignore
  }
}

/** 主窗口监听弹窗关闭事件。返回取消监听函数。 */
export function onSubtitleClosed(handler: () => void): () => void {
  const ch = getChannel();
  const onMessage = (e: MessageEvent) => {
    if (e.data?.type === "close") handler();
  };
  ch.addEventListener("message", onMessage);
  return () => ch.removeEventListener("message", onMessage);
}
