// Theme: light, dark or follow the system. Remembered per user.
export type Theme = "system" | "light" | "dark";

const KEY = "focuside.theme";

function read(): Theme {
  try {
    const t = localStorage.getItem(KEY);
    if (t === "light" || t === "dark" || t === "system") return t;
  } catch {
    // Storage can be unavailable. Follow the system theme.
  }
  return "system";
}

export const theme = $state({ value: read() });

export function applyTheme(t: Theme) {
  theme.value = t;
  if (t === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", t);
  try {
    localStorage.setItem(KEY, t);
  } catch {
    // Not persisted. The choice still applies for this session.
  }
}

/** True when the dark palette is in use right now. */
export function isDark(): boolean {
  if (theme.value === "dark") return true;
  if (theme.value === "light") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}
