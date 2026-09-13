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

export default router;
