import { createRouter, createWebHashHistory } from "vue-router";

// hash 模式在 Tauri 打包后也无需服务器 SPA fallback，避免子路由刷新白屏
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: () => import("../routes/HomePage.vue"),
    },
    {
      path: "/work/:id",
      name: "work-detail",
      component: () => import("../routes/DetailPage.vue"),
    },
    {
      path: "/player",
      name: "player",
      component: () => import("../routes/PlayerPage.vue"),
    },
    {
      path: "/desktop-subtitle",
      name: "desktop-subtitle",
      component: () => import("../routes/DesktopSubtitle.vue"),
    },
    {
      path: "/downloads",
      name: "downloads",
      component: () => import("../routes/DownloadsPage.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../routes/SettingsPage.vue"),
    },
    {
      path: "/import",
      name: "import",
      component: () => import("../routes/ImportPage.vue"),
    },
  ],
});

// #region debug-point B:router
router.beforeEach((to, from) => {
  fetch("http://127.0.0.1:7777/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sessionId: "startup-blank-pages",
      runId: "pre-fix",
      hypothesisId: "B",
      location: "src/router/index.ts",
      msg: "[DEBUG] router.beforeEach",
      data: { to: { path: to.path, name: to.name }, from: { path: from.path, name: from.name } },
      ts: Date.now(),
    }),
  }).catch(() => {});
});

router.afterEach((to, from, failure) => {
  fetch("http://127.0.0.1:7777/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sessionId: "startup-blank-pages",
      runId: "pre-fix",
      hypothesisId: "B",
      location: "src/router/index.ts",
      msg: "[DEBUG] router.afterEach",
      data: {
        to: { path: to.path, name: to.name, matched: to.matched?.length },
        from: { path: from.path, name: from.name },
        failure: failure ? String(failure) : null,
      },
      ts: Date.now(),
    }),
  }).catch(() => {});
});

router.onError((err) => {
  fetch("http://127.0.0.1:7777/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      sessionId: "startup-blank-pages",
      runId: "pre-fix",
      hypothesisId: "B",
      location: "src/router/index.ts",
      msg: "[DEBUG] router.onError",
      data: { error: String(err) },
      ts: Date.now(),
    }),
  }).catch(() => {});
});
// #endregion

export default router;
