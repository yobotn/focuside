// Editor tabs in the viewer, per project, VS Code style. A single click in the file tree opens
// a preview tab (italic name), which the next preview replaces. A double click, an edit or a
// drag pins it. Each open file is a Doc with its text, the text on disk and the CodeMirror
// state (undo history, cursor). Open tabs, cursor and scroll position are saved per project root
// in `app.editorSaved` (the "editor" field of the state file), apart from the rest of the state.
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import { untrack } from "svelte";
import { api, errText, type FileContent } from "../api";
import { t } from "../i18n";
import { app, type Project, type ViewerHost, type ViewerTarget } from "./app.svelte";
import { dock } from "./dock.svelte";

/** Cursor and scroll position: selection anchor and head, and the first visible position. */
export interface ViewPos {
  anchor: number;
  head: number;
  top: number;
}

export type FileTab = { kind: "file"; path: string; preview: boolean };
export type DiffTab = { kind: "diff"; turnId: string; paths: string[]; index: number; sessionKey: string; preview: boolean };
export type Tab = FileTab | DiffTab;

export const DIFF_KEY = "\u0000diff";

export function tabKey(t: Tab): string {
  return t.kind === "file" ? t.path : DIFF_KEY;
}

/** One open file. */
export class Doc {
  readonly path: string;
  /** As read from disk. Null until loaded. */
  content = $state.raw<FileContent | null>(null);
  /** The editor text. */
  text = $state("");
  /** The text on disk (as last loaded or saved). */
  saved = $state("");
  error = $state<string | null>(null);
  /** Set when the file changed on disk while it had unsaved edits. */
  disk = $state<null | "changed" | "deleted">(null);
  /** Bumped when the text is replaced from outside the editor (load or reload). */
  rev = $state(0);
  /** CodeMirror state while the tab is not shown: keeps the undo history and the cursor. */
  state: EditorState | null = null;
  pos: ViewPos | null = null;
  loaded = false;
  private loading: Promise<void> | null = null;

  constructor(path: string, pos: ViewPos | null = null) {
    this.path = path;
    this.pos = pos;
  }

  get dirty(): boolean {
    return this.content?.kind === "text" && this.text !== this.saved;
  }

  load(projectId: string, force = false): Promise<void> {
    if (this.loaded && !force) return Promise.resolve();
    if (this.loading && !force) return this.loading;
    this.loading = api
      .fsRead(projectId, this.path)
      .then((c) => {
        this.error = null;
        this.take(c);
        this.disk = null;
        this.loaded = true;
      })
      .catch((e) => {
        this.error = errText(e);
      })
      .finally(() => (this.loading = null));
    return this.loading;
  }

  /** Takes the disk content as the text (load, reload, or a silent reload after an outside edit). */
  take(c: FileContent) {
    this.content = c;
    if (c.kind === "text") {
      this.text = c.content;
      this.saved = c.content;
    }
    this.rev++;
  }
}

/** The tabs of one project. */
export class Workspace {
  readonly projectId: string;
  tabs = $state<Tab[]>([]);
  /** Key of the active tab (a path, or DIFF_KEY). */
  active = $state<string | null>(null);
  /** The viewer is hidden (Esc, its close button); the tabs stay. */
  hidden = $state(false);
  docs = $state<Record<string, Doc>>({});
  /** Files in the order they were last shown, newest first (Quick open). */
  recent = $state<string[]>([]);
  /** Closed file tabs, newest last (Reopen closed editor). */
  closed: string[] = [];

  constructor(projectId: string) {
    this.projectId = projectId;
  }

  get current(): Tab | null {
    return this.tabs.find((t) => tabKey(t) === this.active) ?? null;
  }

  index(key: string): number {
    return this.tabs.findIndex((t) => tabKey(t) === key);
  }

  doc(path: string): Doc | undefined {
    return this.docs[path];
  }

  get dirtyDocs(): Doc[] {
    return this.tabs.flatMap((t) => (t.kind === "file" && this.docs[t.path]?.dirty ? [this.docs[t.path]] : []));
  }
}

interface SavedWorkspace {
  tabs: { path: string; preview?: boolean }[];
  active: string | null;
  pos: Record<string, ViewPos>;
  recent: string[];
}

/** A question before closing unsaved files: Save, Do not save, Cancel. */
export interface SaveAsk {
  paths: string[];
  answer: (choice: "save" | "discard" | "cancel") => void;
}

/** Where the editor jumps once the file shows (Quick open ":12", a search result). */
export interface Jump {
  projectId: string;
  path: string;
  line: number;
  col?: number;
  len?: number;
  seq: number;
}

const MAX_TABS_SAVED = 40;
const MAX_RECENT = 50;

class EditorStore implements ViewerHost {
  spaces = $state<Record<string, Workspace>>({});
  ask = $state<SaveAsk | null>(null);
  jump = $state<Jump | null>(null);
  /** Reveal a path in the file tree (breadcrumbs, the tab menu). */
  reveal = $state<{ projectId: string; path: string; seq: number } | null>(null);
  /** The editor view of the active file tab, for palette commands. Not reactive. */
  view: EditorView | null = null;
  private seq = 0;
  private inited = false;

  /** Starts once: registers as the viewer host, follows open projects and disk changes. */
  init() {
    if (this.inited) return;
    this.inited = true;
    app.viewerHost = this;
    $effect.root(() => {
      $effect(() => {
        const list = app.projects;
        const ids = list.map((p) => p.id);
        untrack(() => {
          for (const p of list) if (!this.spaces[p.id]) this.restore(p);
          for (const id of Object.keys(this.spaces)) if (!ids.includes(id)) delete this.spaces[id];
        });
      });
      $effect(() => {
        void app.activeId;
        untrack(() => this.sync());
      });
    });
    listen<{ projectId: string; paths: string[] }>("fs-changed", (e) => this.onDisk(e.payload.projectId, e.payload.paths)).catch(() => {});
    getCurrentWindow()
      .onCloseRequested(async (e) => {
        const dirty = Object.values(this.spaces).flatMap((w) => w.dirtyDocs.map((d) => ({ w, d })));
        if (!dirty.length) return;
        e.preventDefault();
        const choice = await this.askSave(dirty.map((x) => x.d.path));
        if (choice === "cancel") return;
        if (choice === "save") {
          for (const { w, d } of dirty) if (!(await this.saveDoc(w, d))) return;
        }
        app.saveNow();
        await getCurrentWindow().destroy();
      })
      .catch(() => {});
  }

  get(projectId: string | undefined | null): Workspace | undefined {
    return projectId ? this.spaces[projectId] : undefined;
  }

  // Viewer host

  setViewer(v: ViewerTarget) {
    if (!v) {
      const w = this.get(app.activeId);
      if (w) this.hide(w);
      return;
    }
    if (v.kind === "file") this.open(v.projectId, v.path);
    else this.openDiff(v.projectId, v);
  }

  async canCloseProject(projectId: string): Promise<boolean> {
    const w = this.get(projectId);
    if (!w) return true;
    const dirty = w.dirtyDocs;
    if (!dirty.length) return true;
    const choice = await this.askSave(dirty.map((d) => d.path));
    if (choice === "cancel") return false;
    if (choice === "save") for (const d of dirty) if (!(await this.saveDoc(w, d))) return false;
    return true;
  }

  /** Tells the app which tab the viewer shows. */
  private sync() {
    const p = app.active;
    const w = this.get(p?.id);
    const tab = w && !w.hidden ? w.current : null;
    let next: ViewerTarget = null;
    if (p && tab?.kind === "file") next = { kind: "file", projectId: p.id, path: tab.path };
    else if (p && tab?.kind === "diff")
      next = { kind: "diff", projectId: p.id, turnId: tab.turnId, paths: tab.paths, index: tab.index, sessionKey: tab.sessionKey };
    const cur = app.viewer;
    if (JSON.stringify(cur) !== JSON.stringify(next)) app.showViewer(next);
  }

  // Opening and switching

  /**
   * Opens a file. Without `pin` it opens as the preview tab (replacing the old preview), unless
   * it is already open. `line` (1-based) and `col` (0-based, UTF-16) move the cursor there.
   */
  open(projectId: string, path: string, opts: { pin?: boolean; line?: number; col?: number; len?: number } = {}) {
    const w = this.get(projectId);
    if (!w) return;
    let i = w.index(path);
    if (i < 0) {
      const tab: FileTab = { kind: "file", path, preview: !opts.pin };
      const at = w.tabs.findIndex((t) => t.preview && t.kind === "file" && !w.docs[t.path]?.dirty);
      const activeAt = w.index(w.active ?? "");
      if (!opts.pin && at >= 0) {
        const old = w.tabs[at] as FileTab;
        this.dropDoc(w, old.path);
        w.tabs[at] = tab;
      } else {
        w.tabs.splice(activeAt >= 0 ? activeAt + 1 : w.tabs.length, 0, tab);
      }
      if (!w.docs[path]) w.docs[path] = new Doc(path, this.savedPos(projectId, path));
    } else if (opts.pin) {
      w.tabs[i].preview = false;
    }
    this.activate(w, path);
    if (opts.line) this.jump = { projectId, path, line: opts.line, col: opts.col, len: opts.len, seq: ++this.seq };
  }

  openDiff(projectId: string, d: { turnId: string; paths: string[]; index: number; sessionKey: string }) {
    const w = this.get(projectId);
    if (!w) return;
    const tab: DiffTab = { kind: "diff", turnId: d.turnId, paths: d.paths, index: d.index, sessionKey: d.sessionKey, preview: true };
    const i = w.index(DIFF_KEY);
    if (i >= 0) w.tabs[i] = tab;
    else {
      const activeAt = w.index(w.active ?? "");
      w.tabs.splice(activeAt >= 0 ? activeAt + 1 : w.tabs.length, 0, tab);
    }
    this.activate(w, DIFF_KEY);
  }

  /** Moves the diff tab to another file of the turn. */
  stepDiff(w: Workspace, index: number) {
    const i = w.index(DIFF_KEY);
    const tab = w.tabs[i];
    if (tab?.kind !== "diff") return;
    w.tabs[i] = { ...tab, index: Math.max(0, Math.min(tab.paths.length - 1, index)) };
    this.sync();
  }

  activate(w: Workspace, key: string) {
    if (w.index(key) < 0) return;
    w.active = key;
    w.hidden = false;
    if (key !== DIFF_KEY) {
      w.recent = [key, ...w.recent.filter((p) => p !== key)].slice(0, MAX_RECENT);
      w.docs[key]?.load(w.projectId);
    }
    this.sync();
    this.persist();
  }

  /** Ctrl+Page Up and Page Down. */
  cycle(w: Workspace, step: 1 | -1) {
    if (w.tabs.length < 2) return;
    const i = w.index(w.active ?? "");
    const next = w.tabs[(i + step + w.tabs.length) % w.tabs.length];
    this.activate(w, tabKey(next));
  }

  pin(w: Workspace, key: string) {
    const tab = w.tabs[w.index(key)];
    if (tab?.preview) {
      tab.preview = false;
      this.persist();
    }
  }

  move(w: Workspace, key: string, index: number) {
    const from = w.index(key);
    if (from < 0) return;
    const list = [...w.tabs];
    const [tab] = list.splice(from, 1);
    tab.preview = false;
    list.splice(Math.max(0, Math.min(list.length, index)), 0, tab);
    w.tabs = list;
    this.persist();
  }

  hide(w: Workspace) {
    w.hidden = true;
    this.sync();
  }

  /** Shows the viewer again with its tabs (palette "Show editor"). False when no tab is open. */
  show(w: Workspace): boolean {
    if (!w.tabs.length) return false;
    w.hidden = false;
    if (!w.current) w.active = tabKey(w.tabs[w.tabs.length - 1]);
    this.activate(w, w.active!);
    return true;
  }

  // Closing

  /** Closes tabs, asking first about unsaved ones. False when the user cancelled. */
  async close(w: Workspace, keys: string[]): Promise<boolean> {
    const dirty = keys.flatMap((k) => (w.docs[k]?.dirty ? [w.docs[k]] : []));
    if (dirty.length) {
      const choice = await this.askSave(dirty.map((d) => d.path));
      if (choice === "cancel") return false;
      if (choice === "save") for (const d of dirty) if (!(await this.saveDoc(w, d))) return false;
    }
    const order = w.tabs.map(tabKey);
    const activeAt = order.indexOf(w.active ?? "");
    for (const k of keys) {
      const i = w.index(k);
      if (i < 0) continue;
      w.tabs.splice(i, 1);
      if (k !== DIFF_KEY) {
        w.closed = [...w.closed.filter((p) => p !== k), k].slice(-30);
        this.dropDoc(w, k);
      }
    }
    if (!w.current) {
      // The tab to the right of the closed active tab takes its place, else the one left of it.
      const left = order.slice(0, activeAt).reverse().find((k) => w.index(k) >= 0);
      const right = order.slice(activeAt + 1).find((k) => w.index(k) >= 0);
      w.active = right ?? left ?? null;
      if (w.active) this.activate(w, w.active);
    }
    this.sync();
    this.persist();
    return true;
  }

  closeOthers(w: Workspace, key: string) {
    return this.close(w, w.tabs.map(tabKey).filter((k) => k !== key));
  }

  closeRight(w: Workspace, key: string) {
    const i = w.index(key);
    return this.close(w, w.tabs.slice(i + 1).map(tabKey));
  }

  closeSaved(w: Workspace) {
    return this.close(w, w.tabs.map(tabKey).filter((k) => !w.docs[k]?.dirty));
  }

  /** Ctrl+Shift+T: the last closed file opens again, pinned. */
  reopenClosed(w: Workspace) {
    const path = w.closed.pop();
    if (path) this.open(w.projectId, path, { pin: true });
  }

  private dropDoc(w: Workspace, path: string) {
    const d = w.docs[path];
    if (d?.pos) this.rememberPos(w.projectId, path, d.pos);
    delete w.docs[path];
  }

  // Saving

  async saveDoc(w: Workspace, d: Doc): Promise<boolean> {
    if (!d.dirty && !d.disk) return true;
    const text = d.text;
    try {
      await api.fsWrite(w.projectId, d.path, text);
      d.saved = text;
      d.disk = null;
      if (d.content?.kind === "text") d.content = { ...d.content, content: text };
      this.pin(w, d.path);
      return true;
    } catch (e) {
      app.flash(t("viewer.saveFailed", { error: errText(e) }), "error");
      return false;
    }
  }

  /** Mod+S: the active file. */
  async saveActive(): Promise<void> {
    const w = this.get(app.activeId);
    const tab = w?.current;
    if (!w || tab?.kind !== "file") return;
    const d = w.docs[tab.path];
    if (d && (d.dirty || d.disk) && (await this.saveDoc(w, d))) app.flash(t("viewer.saved"));
  }

  /** "Save all": every unsaved file in every project. */
  async saveAll(): Promise<void> {
    let n = 0;
    for (const w of Object.values(this.spaces)) for (const d of w.dirtyDocs) if (await this.saveDoc(w, d)) n++;
    app.flash(n ? t("editor.savedAll", { count: n }) : t("editor.nothingToSave"));
  }

  askSave(paths: string[]): Promise<"save" | "discard" | "cancel"> {
    return new Promise((resolve) => {
      this.ask = {
        paths,
        answer: (choice) => {
          this.ask = null;
          resolve(choice);
        },
      };
    });
  }

  // Disk changes

  /** A file changed on disk: reload it when it has no unsaved edits, else ask. */
  private async onDisk(projectId: string, paths: string[]) {
    const w = this.get(projectId);
    if (!w) return;
    for (const path of paths) {
      const d = w.docs[path];
      if (!d?.loaded) continue;
      const c = await api.fsRead(projectId, path).catch(() => null);
      if (!c || w.docs[path] !== d) continue;
      const sameText = c.kind === "text" && d.content?.kind === "text" && c.content === d.saved;
      if (sameText || (c.kind === "missing" && d.content?.kind === "missing")) continue;
      if (!d.dirty) {
        d.take(c);
        d.disk = null;
      } else if (c.kind === "text" && c.content === d.text) {
        // The disk now has the editor text.
        d.saved = c.content;
        d.content = c;
        d.disk = null;
      } else d.disk = c.kind === "missing" ? "deleted" : "changed";
    }
  }

  /** "Load from disk": drops the unsaved edits. */
  async reload(w: Workspace, d: Doc) {
    await d.load(w.projectId, true);
  }

  /** "Keep mine": the editor text stays and counts as unsaved against the new disk text. */
  async keepMine(w: Workspace, d: Doc) {
    const c = await api.fsRead(w.projectId, d.path).catch(() => null);
    if (c?.kind === "text") {
      d.saved = c.content;
      d.content = c;
    } else if (c?.kind === "missing") d.saved = "\u0000";
    d.disk = null;
  }

  // Positions and persistence

  /** The editor reports the cursor and scroll position of a file. */
  setPos(projectId: string, path: string, pos: ViewPos) {
    const d = this.get(projectId)?.docs[path];
    if (d) d.pos = pos;
    this.persist();
  }

  /** Shows the file tree with the path selected (breadcrumbs, the tab menu). */
  requestReveal(projectId: string, path: string) {
    app.layout.sideView = "files";
    dock.open("side");
    this.reveal = { projectId, path, seq: ++this.seq };
  }

  private saved(): Record<string, SavedWorkspace> {
    const raw = app.editorSaved;
    return raw && typeof raw === "object" ? (raw as Record<string, SavedWorkspace>) : {};
  }

  private savedPos(projectId: string, path: string): ViewPos | null {
    const root = app.projects.find((p) => p.id === projectId)?.root;
    const pos = root ? this.saved()[root]?.pos?.[path] : undefined;
    return pos && typeof pos.anchor === "number" ? pos : null;
  }

  private rememberPos(projectId: string, path: string, pos: ViewPos) {
    const root = app.projects.find((p) => p.id === projectId)?.root;
    if (!root) return;
    const all = { ...this.saved() };
    const ws = all[root] ?? { tabs: [], active: null, pos: {}, recent: [] };
    all[root] = { ...ws, pos: { ...ws.pos, [path]: pos } };
    app.editorSaved = all;
  }

  private restore(p: Project) {
    const w = new Workspace(p.id);
    const s = this.saved()[p.root];
    if (s && Array.isArray(s.tabs)) {
      for (const tab of s.tabs.slice(0, MAX_TABS_SAVED)) {
        if (typeof tab?.path !== "string" || w.index(tab.path) >= 0) continue;
        w.tabs.push({ kind: "file", path: tab.path, preview: !!tab.preview });
        const pos = s.pos?.[tab.path];
        w.docs[tab.path] = new Doc(tab.path, pos && typeof pos.anchor === "number" ? pos : null);
      }
      w.recent = Array.isArray(s.recent) ? s.recent.filter((x) => typeof x === "string").slice(0, MAX_RECENT) : [];
      w.active = s.active && w.index(s.active) >= 0 ? s.active : w.tabs[0] ? tabKey(w.tabs[0]) : null;
    }
    this.spaces[p.id] = w;
    if (w.active) w.docs[w.active]?.load(p.id);
    if (app.activeId === p.id) this.sync();
  }

  private persistTimer: ReturnType<typeof setTimeout> | null = null;

  /** Saves open tabs, positions and recent files per project root, shortly after a change. */
  persist() {
    if (this.persistTimer) clearTimeout(this.persistTimer);
    this.persistTimer = setTimeout(() => {
      this.persistTimer = null;
      const all = { ...this.saved() };
      for (const w of Object.values(this.spaces)) {
        const root = app.projects.find((p) => p.id === w.projectId)?.root;
        if (!root) continue;
        const files = w.tabs.filter((t): t is FileTab => t.kind === "file").slice(0, MAX_TABS_SAVED);
        const pos: Record<string, ViewPos> = {};
        // Positions of closed files stay, so a file opens where it was left.
        for (const [k, v] of Object.entries(all[root]?.pos ?? {}).slice(-200)) pos[k] = v;
        for (const f of files) {
          const d = w.docs[f.path];
          if (d?.pos) pos[f.path] = { ...d.pos };
        }
        all[root] = {
          tabs: files.map((f) => (f.preview ? { path: f.path, preview: true } : { path: f.path })),
          active: w.active && w.active !== DIFF_KEY ? w.active : (files[0]?.path ?? null),
          pos,
          recent: [...w.recent],
        };
      }
      app.editorSaved = all;
      app.save();
    }, 500);
  }
}

export const editor = new EditorStore();

/** Names for tabs: the file name, plus as many parent folders as it takes to tell apart tabs
 * with the same name. */
export function tabNames(tabs: Tab[]): Map<string, { name: string; dir: string }> {
  const out = new Map<string, { name: string; dir: string }>();
  const files = tabs.filter((t): t is FileTab => t.kind === "file");
  const base = (p: string) => p.split("/").pop() ?? p;
  const dirs = (p: string) => p.split("/").slice(0, -1);
  const tail = (parts: string[], n: number) => parts.slice(-n).join("/");
  for (const f of files) {
    const name = base(f.path);
    const others = files.filter((o) => o !== f && base(o.path) === name).map((o) => dirs(o.path));
    const parts = dirs(f.path);
    let dir = "";
    if (others.length && parts.length) {
      let n = 1;
      while (n < parts.length && others.some((o) => tail(o, n) === tail(parts, n))) n++;
      dir = tail(parts, n);
    }
    out.set(f.path, { name, dir });
  }
  return out;
}
