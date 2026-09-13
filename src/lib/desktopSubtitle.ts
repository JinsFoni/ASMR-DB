// 桌面悬浮字幕窗口管理（类似桌面歌词）
import { currentMonitor } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emitTo } from "@tauri-apps/api/event";

const WINDOW_LABEL = "desktop-subtitle";
const WIDTH = 560;
const HEIGHT = 150;

let win: WebviewWindow | null = null;

/** 计算屏幕底部居中位置。 */
async function bottomCenterPos() {
  try {
    const monitor = await currentMonitor();
    if (monitor) {
      const { width, height } = monitor.size;
      const { x, y } = monitor.position;
      return {
        x: Math.round(x + (width - WIDTH) / 2),
        y: Math.round(y + height - HEIGHT - 40),
      };
    }
  } catch {
    // ignore
  }
  return { x: 100, y: 80 };
}

/** 打开（或聚焦）桌面字幕窗口。 */
export async function openDesktopSubtitleWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(WINDOW_LABEL);
  if (existing) {
    win = existing;
    await existing.show();
    await existing.setFocus();
    return;
  }
  const pos = await bottomCenterPos();
  win = new WebviewWindow(WINDOW_LABEL, {
    url: "#/desktop-subtitle",
    title: "桌面字幕",
    width: WIDTH,
    height: HEIGHT,
    x: pos.x,
    y: pos.y,
    decorations: false,
    transparent: true,
    alwaysOnTop: true,
    resizable: false,
    skipTaskbar: true,
    focus: false,
  });
  // 等待窗口创建完成，避免事件丢失
  await new Promise((r) => setTimeout(r, 120));
}

/** 关闭桌面字幕窗口。 */
export async function closeDesktopSubtitleWindow(): Promise<void> {
  const target = win ?? (await WebviewWindow.getByLabel(WINDOW_LABEL));
  if (target) {
    try {
      await target.close();
    } catch {
      // already closed
    }
  }
  win = null;
}

/** 向桌面字幕窗口推送当前字幕文本。 */
export function pushDesktopSubtitleText(text: string): void {
  emitTo(WINDOW_LABEL, "desktop-subtitle:text", { text }).catch(() => {});
}
