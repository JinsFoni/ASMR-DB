<script setup lang="ts">
import { onMounted } from "vue";
import { RouterLink, useRoute } from "vue-router";
import {
  Library,
  Plus,
} from "lucide-vue-next";
import { useWorksStore } from "../../stores/works";

const route = useRoute();
const worksStore = useWorksStore();

onMounted(() => {
  worksStore.loadTags();
});

const links = [
  { to: "/", label: "作品库", match: (p: string) => p === "/" },
  { to: "/dlsite", label: "榜单", match: (p: string) => p.startsWith("/dlsite") },
  { to: "/asmr", label: "在线", match: (p: string) => p.startsWith("/asmr") },
  { to: "/import", label: "导入", match: (p: string) => p === "/import" },
  { to: "/downloads", label: "下载", match: (p: string) => p === "/downloads" },
  { to: "/translate", label: "翻译", match: (p: string) => p === "/translate" },
  { to: "/settings", label: "设置", match: (p: string) => p === "/settings" },
];
</script>

<template>
  <!-- ============ 灵动岛式毛玻璃胶囊导航 ============ -->
  <header class="nav-pill-wrap">
    <div class="nav-pill glass">
      <RouterLink to="/" class="nav-brand" title="ASMR DB">
        <div class="brand-ico">
          <Library :size="17" />
        </div>
      </RouterLink>

      <nav class="nav-links">
        <RouterLink
          v-for="l in links"
          :key="l.to"
          :to="l.to"
          :class="{ on: l.match(route.path) }"
        >
          {{ l.label }}
        </RouterLink>
      </nav>

      <!-- 筛选入口已迁移至作品库筛选行（WorkFilters.vue 的 chips 行） -->
      <div class="relative ml-auto flex items-center gap-2">
        <RouterLink
          to="/import"
          class="nav-cta"
          :class="{ hidden: route.name === 'import' }"
        >
          <Plus :size="14" /> 导入作品
        </RouterLink>
      </div>
    </div>
  </header>
</template>

<style scoped>
.nav-pill-wrap {
  position: sticky;
  top: 14px;
  z-index: 60;
  max-width: 760px;
  margin: 14px auto 0;
  padding: 0 20px;
}
.nav-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 54px;
  border-radius: 999px;
  box-shadow:
    0 8px 30px -8px rgba(0, 0, 0, 0.6),
    inset 0 0 0 0.5px rgba(255, 255, 255, 0.14);
  padding: 0 10px 0 12px;
}
html[data-theme="light"] .nav-pill,
html[data-theme="sakura"] .nav-pill {
  box-shadow:
    0 8px 30px -8px rgba(0, 0, 0, 0.18),
    inset 0 0 0 0.5px rgba(0, 0, 0, 0.06);
}
.nav-brand {
  display: flex;
  align-items: center;
  margin-right: 10px;
  flex-shrink: 0;
}
.brand-ico {
  width: 34px;
  height: 34px;
  border-radius: 11px;
  background: var(--blue);
  color: #fff;
  display: grid;
  place-items: center;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.45);
}
.nav-links {
  display: flex;
  align-items: center;
  gap: 2px;
  overflow-x: auto;
  scrollbar-width: none;
}
.nav-links::-webkit-scrollbar {
  display: none;
}
.nav-links a {
  font-size: 13px;
  font-weight: 500;
  color: var(--label-2);
  text-decoration: none;
  padding: 7px 13px;
  border-radius: 999px;
  white-space: nowrap;
  transition:
    background 0.2s,
    color 0.2s;
}
.nav-links a:hover {
  color: var(--label);
  background: var(--fill-1);
}
.nav-links a.on {
  color: #fff;
  background: #48484c;
}
html[data-theme="light"] .nav-links a.on,
html[data-theme="sakura"] .nav-links a.on {
  color: #fff;
  background: var(--blue);
}
.nav-mini-btn {
  height: 36px;
  padding: 0 14px;
  border: none;
  border-radius: 999px;
  background: var(--fill-1);
  color: var(--label-2);
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
  white-space: nowrap;
}
.nav-mini-btn:hover,
.nav-mini-btn.on {
  background: var(--fill-3);
  color: var(--label);
}
.nav-cta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 18px;
  border: none;
  border-radius: 999px;
  background: var(--blue);
  color: #fff;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  text-decoration: none;
  transition:
    background 0.2s,
    transform 0.2s;
  white-space: nowrap;
}
.nav-cta:hover {
  background: var(--blue-deep);
}

@media (max-width: 900px) {
  .nav-pill-wrap {
    top: 10px;
    margin-top: 10px;
    padding: 0 14px;
  }
}
</style>
