export type ThemeName = "dark" | "light" | "sakura";

export function applyTheme(theme: string) {
  const validTheme = (theme === "light" || theme === "sakura" ? theme : "dark") as ThemeName;
  document.documentElement.setAttribute("data-theme", validTheme);
  try {
    localStorage.setItem("dlsite_theme", validTheme);
  } catch (_) {}
}

export function getStoredTheme(): ThemeName {
  try {
    const t = localStorage.getItem("dlsite_theme");
    if (t === "light" || t === "sakura" || t === "dark") return t;
  } catch (_) {}
  return "dark";
}
