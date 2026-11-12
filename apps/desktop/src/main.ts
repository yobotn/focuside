import { mount } from "svelte";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/cascadia-code/400.css";
import "./app.css";
import App from "./App.svelte";
import { applyTheme, theme } from "./lib/theme.svelte";

applyTheme(theme.value);

// Dev builds show uncaught errors on screen; the webview console is not visible there.
if (import.meta.env.DEV) {
  const show = (msg: string) => {
    const el = document.createElement("pre");
    el.style.cssText = "position:fixed;left:8px;right:8px;bottom:8px;z-index:9999;max-height:40vh;overflow:auto;background:#7f1d1d;color:#fff;padding:8px;font:12px monospace;white-space:pre-wrap";
    el.textContent = msg;
    document.body.appendChild(el);
  };
  const log = (m: string) => import("@tauri-apps/api/core").then(({ invoke }) => invoke("debug_log", { message: m })).catch(() => {});
  const origError = console.error;
  console.error = (...a: unknown[]) => {
    log(a.map((x) => (x instanceof Error ? `${x.name}: ${x.message} | ${x.stack}` : typeof x === "object" ? JSON.stringify(x) : String(x))).join(" "));
    origError(...a);
  };
  const origWarn = console.warn;
  console.warn = (...a: unknown[]) => {
    log("warn: " + a.map(String).join(" "));
    origWarn(...a);
  };
  window.addEventListener("error", (e) => {
    log(String(e.error?.stack ?? e.message));
    show(String(e.error?.stack ?? e.message));
  });
  window.addEventListener("unhandledrejection", (e) => {
    log("Unhandled: " + String(e.reason?.stack ?? e.reason));
    show("Unhandled: " + String(e.reason?.stack ?? e.reason));
  });
}

const app = mount(App, { target: document.getElementById("app")! });

export default app;
