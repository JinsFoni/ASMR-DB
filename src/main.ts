import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import "./style.css";

// #region debug-point D:global-errors
window.addEventListener("error", (ev) => {
  fetch("http://127.0.0.1:7777/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sessionId: "startup-blank-pages",
      runId: "pre-fix",
      hypothesisId: "D",
      location: "src/main.ts",
      msg: "[DEBUG] window.error",
      data: {
        message: (ev as ErrorEvent).message,
        filename: (ev as ErrorEvent).filename,
        lineno: (ev as ErrorEvent).lineno,
        colno: (ev as ErrorEvent).colno,
      },
      ts: Date.now(),
    }),
  }).catch(() => {});
});
window.addEventListener("unhandledrejection", (ev) => {
  fetch("http://127.0.0.1:7777/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sessionId: "startup-blank-pages",
      runId: "pre-fix",
      hypothesisId: "D",
      location: "src/main.ts",
      msg: "[DEBUG] window.unhandledrejection",
      data: { reason: String((ev as PromiseRejectionEvent).reason) },
      ts: Date.now(),
    }),
  }).catch(() => {});
});
// #endregion

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
