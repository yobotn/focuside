// The terminal of each agent pane. It lives as long as its session, not as long as the view:
// switching sessions moves the terminal element out of the page and back, so the TUI keeps
// its screen and keeps running.
//
// FocusIDE writes into the agent's PTY only what xterm.js reports as input: the user's keys
// and paste, and xterm's own answers to terminal queries (cursor position). It never types a
// prompt or a command into an agent pane. Two exceptions, both on a user action: the plan
// mode switch of a Claude pane sends Shift+Tab key sequences on a click (see pane.rs,
// `pane_mode_key`), and a paste with no text on the clipboard sends the program's image paste
// key (see tui.ts, `imagePasteKey`).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { t } from "../i18n";
import { isWindows } from "../platform";
import { CLAUDE_STATUS_ROWS, claudeMode, imagePasteKey } from "../tui";
import { decode, newTerminal, rightClickPaste } from "../xterm";

export class PaneTerm {
  readonly key: string;
  readonly projectId: string;
  readonly el: HTMLDivElement;
  readonly term: Terminal;
  readonly fit = new FitAddon();
  ptyId: string | null = null;
  /** The TUI exited. Enter starts it again. */
  exited = $state(false);
  private opened = false;
  private starting = false;
  /** Gets the permission mode that the TUI's status line shows (Claude panes), or null when
   * no status line can be read. Called when it changes. */
  onMode: ((mode: string | null) => void) | null = null;
  /** Gets the terminal title the TUI sets (OSC 0 or OSC 2). */
  onTitle: ((title: string) => void) | null = null;
  private modeTimer: ReturnType<typeof setTimeout> | null = null;
  private lastMode: string | null | undefined = undefined;

  constructor(key: string, projectId: string, agent: string) {
    this.key = key;
    this.projectId = projectId;
    this.el = document.createElement("div");
    this.el.className = "pane-term";
    this.el.style.height = "100%";
    const imageKey = imagePasteKey(agent, isWindows);
    this.term = newTerminal(10000, { imageKey });
    this.term.loadAddon(this.fit);
    rightClickPaste(this.term, this.el, { imageKey });
    this.term.onTitleChange((title) => this.onTitle?.(title));
    this.term.onData((data) => {
      if (this.exited) {
        if (data === "\r") this.start();
        return;
      }
      if (this.ptyId) invoke("pty_write", { id: this.ptyId, data }).catch(() => {});
    });
    this.term.onResize(({ cols, rows }) => {
      if (this.ptyId) invoke("pty_resize", { id: this.ptyId, cols, rows }).catch(() => {});
    });
  }

  /** Puts the terminal into `host`. The first time, the agent's TUI starts. */
  attach(host: HTMLElement) {
    host.appendChild(this.el);
    if (!this.opened) {
      this.term.open(this.el);
      this.opened = true;
    }
    this.resize();
    if (!this.ptyId && !this.exited) this.start();
    this.term.focus();
  }

  detach() {
    this.el.remove();
  }

  /** New output arrived: read the status line shortly after the screen settles. */
  output(bytes: Uint8Array) {
    this.term.write(bytes);
    if (!this.onMode || this.modeTimer) return;
    this.modeTimer = setTimeout(() => {
      this.modeTimer = null;
      const m = this.shownMode();
      if (m !== this.lastMode) {
        this.lastMode = m;
        this.onMode?.(m);
      }
    }, 150);
  }

  /** The mode in the bottom lines of the screen, where Claude draws its status line. Null
   * when no line matches (see tui.ts). */
  shownMode(): string | null {
    try {
      const buf = this.term.buffer.active;
      const last = buf.baseY + this.term.rows - 1;
      const lines: string[] = [];
      for (let y = last; y > last - CLAUDE_STATUS_ROWS && y >= 0; y--) lines.push(buf.getLine(y)?.translateToString(true) ?? "");
      return claudeMode(lines);
    } catch {
      return null;
    }
  }

  /** Waits until the shown mode is not `from` any more, at most `ms`. Returns the shown mode. */
  async waitForMode(from: string | null, ms: number): Promise<string | null> {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      await new Promise((r) => setTimeout(r, 50));
      const m = this.shownMode();
      if (m !== from) {
        // Let the status line finish drawing, then read it again.
        await new Promise((r) => setTimeout(r, 120));
        return this.shownMode();
      }
    }
    return this.shownMode();
  }

  resize() {
    if (this.el.isConnected && this.el.clientWidth > 0) this.fit.fit();
  }

  focus() {
    this.term.focus();
  }

  async start() {
    if (this.starting) return;
    this.starting = true;
    this.exited = false;
    this.resize();
    try {
      const id = await invoke<string>("pty_open", {
        projectId: this.projectId,
        cols: this.term.cols,
        rows: this.term.rows,
        shell: null,
        session: this.key,
      });
      this.ptyId = id;
      flush(this, id);
    } catch (e) {
      this.exited = true;
      this.term.writeln(t("pane.failed", { error: String(e) }));
    } finally {
      this.starting = false;
    }
  }

  /** Clears the screen and starts the TUI again (a fresh one after a failed resume). */
  restart() {
    this.term.reset();
    this.start();
  }

  onExit() {
    this.ptyId = null;
    this.exited = true;
    this.term.writeln("\r\n" + t("pane.exitedTerm"));
  }

  dispose() {
    if (this.modeTimer) clearTimeout(this.modeTimer);
    this.onMode = null;
    this.onTitle = null;
    this.el.remove();
    this.term.dispose();
  }
}

const terms = new Map<string, PaneTerm>();

// Output can arrive before pty_open returns its id. It waits here, so the TUI's first bytes
// (and ConPTY's cursor position request) are not lost.
const early = new Map<string, { chunks: Uint8Array[]; exited: boolean }>();

function flush(p: PaneTerm, id: string) {
  const e = early.get(id);
  early.delete(id);
  if (!e) return;
  for (const c of e.chunks) p.output(c);
  if (e.exited) p.onExit();
}

function owner(id: string): PaneTerm | undefined {
  for (const p of terms.values()) if (p.ptyId === id) return p;
  return undefined;
}

function hold(id: string): { chunks: Uint8Array[]; exited: boolean } {
  let e = early.get(id);
  if (!e) {
    e = { chunks: [], exited: false };
    early.set(id, e);
    // PTYs of the terminal drawer also land here; drop what nobody claims.
    setTimeout(() => early.delete(id), 10000);
  }
  return e;
}

let listening = false;

function listenOnce() {
  if (listening) return;
  listening = true;
  listen<{ id: string; data: string }>("pty", (e) => {
    const bytes = decode(e.payload.data);
    const p = owner(e.payload.id);
    if (p) p.output(bytes);
    else if ([...terms.values()].some((x) => x.ptyId === null)) hold(e.payload.id).chunks.push(bytes);
  });
  listen<{ id: string }>("pty-exit", (e) => {
    const p = owner(e.payload.id);
    if (p) p.onExit();
    else if ([...terms.values()].some((x) => x.ptyId === null)) hold(e.payload.id).exited = true;
  });
}

export const paneTerms = {
  /** The terminal of a pane session, created on first use. */
  get(key: string, projectId: string, agent: string): PaneTerm {
    listenOnce();
    let p = terms.get(key);
    if (!p) {
      p = new PaneTerm(key, projectId, agent);
      terms.set(key, p);
    }
    return p;
  },
  /** The terminal of a pane session, if it was created. */
  find(key: string): PaneTerm | undefined {
    return terms.get(key);
  },
  /** Called when the session closes. Rust has killed the TUI by then. */
  dispose(key: string) {
    terms.get(key)?.dispose();
    terms.delete(key);
  },
};
