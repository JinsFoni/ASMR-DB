<script setup lang="ts">
import { CheckCircle2, Info, TriangleAlert } from "lucide-vue-next";
import { dismiss, toasts, type ToastKind } from "../../lib/toast";

const icons: Record<ToastKind, unknown> = {
  success: CheckCircle2,
  error: TriangleAlert,
  info: Info,
};
const colors: Record<ToastKind, string> = {
  success: "var(--green)",
  error: "#ff6961",
  info: "var(--blue-deep)",
};
</script>

<template>
  <div class="toast-host">
    <TransitionGroup name="toast">
      <div v-for="t in toasts" :key="t.id" class="toast glass" @click="dismiss(t.id)">
        <component :is="icons[t.kind]" :size="14" :style="{ color: colors[t.kind] }" />
        <span>{{ t.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  /* 底部居中、MiniPlayer 上方，避开顶部悬浮导航 */
  bottom: 118px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 100;
  display: flex;
  flex-direction: column-reverse;
  align-items: center;
  gap: 8px;
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: min(520px, calc(100vw - 40px));
  padding: 9px 16px;
  border-radius: 999px;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--label);
  cursor: pointer;
  box-shadow: var(--shadow-float);
}
.toast span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 进出场：底部淡入上滑 */
.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.28s ease, transform 0.28s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.toast-enter-from {
  opacity: 0;
  transform: translateY(12px);
}
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
