// The terminal of each agent pane. It lives as long as its session, not as long as the view:
// switching sessions moves the terminal element out of the page and back, so the TUI keeps
// its screen and keeps running.
//
// FocusIDE writes into the agent's PTY only what xterm.js reports as input: the user's keys
// and paste, and xterm's own answers to terminal queries (cursor position). It never types a
// prompt or a command into an agent pane.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { t } from "../i18n";
import { decode, newTerminal } from "../xterm";

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

  constructor(key: string, projectId: string) {
    this.key = key;
    this.projectId = projectId;
    this.el = document.createElement("div");
    this.el.className = "pane-term";
    this.el.style.height = "100%";
    this.term = newTerminal(10000);
    this.term.loadAddon(this.fit);
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

  onExit() {
    this.ptyId = null;
    this.exited = true;
    this.term.writeln("\r\n" + t("pane.exitedTerm"));
  }

  dispose() {
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
  for (const c of e.chunks) p.term.write(c);
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
    if (p) p.term.write(bytes);
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
  get(key: string, projectId: string): PaneTerm {
    listenOnce();
    let p = terms.get(key);
    if (!p) {
      p = new PaneTerm(key, projectId);
      terms.set(key, p);
    }
    return p;
  },
  /** Called when the session closes. Rust has killed the TUI by then. */
  dispose(key: string) {
    terms.get(key)?.dispose();
    terms.delete(key);
  },
};
