// The status bar at the bottom of the window (StatusBar.svelte) shows a list of items. Other
// parts of the app add their own item with `statusBar.add(...)`, for example the git branch:
//
//   statusBar.add({ id: "branch", align: "start", order: 5, text: () => app.active?.branch ?? "", run: () => ... });
//
// The functions are read while the bar renders, so items follow the app state. An item with
// empty text is hidden. Built-in items: the agent state of the active session and the files it
// changed (start), the cursor and the language of the open file (end).
import type { Component } from "svelte";
import { languageName } from "../editor";
import { gotoLine } from "../findPanel";
import { t } from "../i18n";
import { sessionFiles, splitDuration } from "../review";
import { app } from "./app.svelte";
import { dock } from "./dock.svelte";
import { editor } from "./editor.svelte";

export interface StatusItem {
  id: string;
  /** "start": the left side (right side in right-to-left text). "end": the other side. */
  align: "start" | "end";
  /** Lower comes first. Built-in: agent 10 and files 20 (start), cursor 10 and language 20 (end). */
  order: number;
  text: () => string;
  title?: () => string;
  tone?: () => "muted" | "accent" | "warn" | "danger" | undefined;
  /** A Phosphor icon (or any component that takes `size`). */
  icon?: () => Component<{ size?: number }> | null;
  /** The item is a button. */
  run?: () => void;
}

/** "12s", "3m 05s", "1h 02m". */
export function formatDuration(ms: number): string {
  const { h, m, s } = splitDuration(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  if (h) return t("time.hours", { h, m: pad(m) });
  if (m) return t("time.minutes", { m, s: pad(s) });
  return t("time.seconds", { s });
}

class StatusBarStore {
  items = $state<StatusItem[]>([]);
  /** The time, every second while a session works (elapsed time in the bar and the tabs). */
  now = $state(Date.now());
  private clock: ReturnType<typeof setInterval> | null = null;
  private started = false;

  /** Adds or replaces an item. Returns a function that removes it. */
  add(item: StatusItem): () => void {
    this.items = [...this.items.filter((x) => x.id !== item.id), item];
    return () => this.remove(item.id);
  }

  remove(id: string) {
    this.items = this.items.filter((x) => x.id !== id);
  }

  side(align: "start" | "end"): StatusItem[] {
    return this.items.filter((x) => x.align === align).sort((a, b) => a.order - b.order);
  }

  /** Starts the clock and the built-in items. Runs once (App.svelte). */
  start() {
    if (this.started) return;
    this.started = true;
    $effect.root(() => {
      $effect(() => {
        const busy = app.projects.some((p) => p.sessions.some((s) => s.busy));
        if (busy && !this.clock) {
          this.now = Date.now();
          this.clock = setInterval(() => (this.now = Date.now()), 1000);
        } else if (!busy && this.clock) {
          clearInterval(this.clock);
          this.clock = null;
        }
      });
    });
    builtIns(this);
  }
}

export const statusBar = new StatusBarStore();

/** The file the viewer shows, when it is a file tab of the active project. */
function shownFile(): string | null {
  const v = app.viewer;
  return v?.kind === "file" && v.projectId === app.activeId ? v.path : null;
}

/** The state of a session in words, for the bar and the tab tooltip. */
export function stateWords(s: NonNullable<typeof app.session>, now: number): string {
  switch (s.tabState) {
    case "working":
      return t("status.working", { time: formatDuration(now - (s.busySince ?? now)) });
    case "waiting":
      return s.waiting === "permission" ? t("queue.permission") : t("queue.plan");
    case "failed":
      return s.status === "failed" ? t("status.startFailed") : t("queue.failed");
    case "starting":
      return t("status.starting");
    case "dormant":
      return t("session.selectToResume");
    case "review":
      return t("status.review", { count: s.unreviewed });
    case "done":
      return s.lastRun ? t("status.doneIn", { time: formatDuration(s.lastRun.end - s.lastRun.start) }) : t("status.done");
    default:
      return t("status.ready");
  }
}

function builtIns(bar: StatusBarStore) {
  bar.add({
    id: "agent",
    align: "start",
    order: 10,
    text: () => {
      const s = app.session;
      return s ? t("status.agent", { agent: app.agentName(s.agent), state: stateWords(s, bar.now) }) : "";
    },
    title: () => t("status.agentHint"),
    tone: () => {
      const st = app.session?.tabState;
      return st === "working" || st === "review" || st === "done" ? "accent" : st === "waiting" ? "warn" : st === "failed" ? "danger" : "muted";
    },
    run: () => dock.show("session"),
  });
  bar.add({
    id: "files",
    align: "start",
    order: 20,
    text: () => {
      const s = app.session;
      const n = s ? sessionFiles(s.entries).length : 0;
      return n ? t("status.files", { count: n }) : "";
    },
    title: () => {
      const n = app.session?.unreviewed ?? 0;
      return n ? t("status.filesHintOpen", { count: n }) : t("status.filesHint");
    },
    run: () => app.reviewSession(),
  });
  bar.add({
    id: "cursor",
    align: "end",
    order: 10,
    text: () => {
      const path = shownFile();
      const c = editor.cursor;
      return path && c && c.path === path ? t("status.cursor", { line: c.line, col: c.col }) : "";
    },
    title: () => t("cmd.gotoLine"),
    run: () => {
      const v = editor.view;
      if (!v) return;
      v.focus();
      gotoLine(v);
    },
  });
  bar.add({
    id: "language",
    align: "end",
    order: 20,
    text: () => {
      const path = shownFile();
      return path ? (languageName(path) ?? t("status.plainText")) : "";
    },
  });
}
