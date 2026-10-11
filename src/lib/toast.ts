import { ref } from "vue";

export type ToastKind = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  message: string;
  kind: ToastKind;
}

/** 全局轻提示状态：ToastHost 渲染，任意模块 showToast() 触发 */
export const toasts = ref<ToastItem[]>([]);
let nextId = 1;

function push(message: string, kind: ToastKind) {
  const id = nextId++;
  toasts.value.push({ id, message, kind });
  // 停留 3s 后淡出（与 CSS 动画时长配合）
  window.setTimeout(() => dismiss(id), 3000);
}

export function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

export function showToast(message: string, kind: ToastKind = "info") {
  push(message, kind);
}
