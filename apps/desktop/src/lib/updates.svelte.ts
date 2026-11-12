// App updates from GitHub Releases (signed latest.json), and links to GitHub issues.
// The check runs a while after start and then every few hours. Nothing installs without a click:
// the update downloads in the background, then the user picks when to restart.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";

export const REPO = "https://github.com/yobotn/focuside";

type State =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "none" }
  | { kind: "downloading"; version: string; percent: number | null }
  | { kind: "ready"; version: string; notes: string }
  | { kind: "installing"; version: string }
  | { kind: "error"; message: string };

class Updates {
  state = $state<State>({ kind: "idle" });
  private update: Update | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;

  /** Starts the background checks. Off in development builds. */
  start(auto: boolean) {
    if (import.meta.env.DEV || this.timer) return;
    if (!auto) return;
    setTimeout(() => this.check(false), 15_000);
    this.timer = setInterval(() => this.check(false), 6 * 60 * 60 * 1000);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }

  /** Checks for an update and downloads it. `manual` reports "up to date" and errors. */
  async check(manual: boolean) {
    if (this.state.kind === "checking" || this.state.kind === "downloading" || this.state.kind === "installing") return;
    if (this.state.kind === "ready") return;
    if (manual) this.state = { kind: "checking" };
    try {
      const u = await check();
      if (!u) {
        this.state = manual ? { kind: "none" } : { kind: "idle" };
        return;
      }
      this.update = u;
      let total = 0;
      let got = 0;
      this.state = { kind: "downloading", version: u.version, percent: null };
      await u.download((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        else if (e.event === "Progress") {
          got += e.data.chunkLength;
          if (total) this.state = { kind: "downloading", version: u.version, percent: Math.round((got / total) * 100) };
        }
      });
      this.state = { kind: "ready", version: u.version, notes: u.body ?? "" };
    } catch (e) {
      this.state = manual ? { kind: "error", message: String(e) } : { kind: "idle" };
    }
  }

  /** Installs the downloaded update and restarts the app. */
  async installAndRestart() {
    const u = this.update;
    if (!u || this.state.kind !== "ready") return;
    this.state = { kind: "installing", version: u.version };
    try {
      await u.install();
      await relaunch();
    } catch (e) {
      this.state = { kind: "error", message: String(e) };
    }
  }

  releaseNotes(version: string) {
    openUrl(`${REPO}/releases/tag/v${version}`).catch(() => {});
  }
}

export const updates = new Updates();

/** Opens a new GitHub issue with the app version and OS filled in. */
export function reportIssue(kind: "bug" | "feature", info: { version?: string; os?: string; details?: string } = {}) {
  const q = new URLSearchParams({ template: kind === "bug" ? "bug.yml" : "feature.yml" });
  if (kind === "bug") {
    if (info.version) q.set("version", info.version);
    if (info.os) q.set("os", info.os);
    if (info.details) q.set("logs", info.details.slice(0, 1500));
  }
  openUrl(`${REPO}/issues/new?${q}`).catch(() => {});
}
