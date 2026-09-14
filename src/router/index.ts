import { createRouter, createWebHashHistory } from "vue-router";

// hash 模式部署在任意静态目录/子路径下都无需服务器 SPA fallback
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: () => import("../routes/HomePage.vue"),
    },
    {
      path: "/dlsite",
      name: "dlsite",
      component: () => import("../routes/DlsitePage.vue"),
    },
    {
      path: "/asmr",
      name: "asmr",
      component: () => import("../routes/AsmrOnlinePage.vue"),
    },
    {
      path: "/asmr/work/:id",
      name: "asmr-work-detail",
      component: () => import("../routes/AsmrOnlineDetailPage.vue"),
    },
    {
      path: "/dlsite/work/:rj",
      name: "dlsite-work-detail",
      component: () => import("../routes/DlsiteDetailPage.vue"),
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
      path: "/translate",
      name: "translate",
      component: () => import("../routes/TranslatePage.vue"),
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

export default router;
