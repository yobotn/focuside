// App state: open projects, sessions, the active view, the waiting queue, settings, and the
// actions the UI calls. Persisted through ui_state_save (recent projects, session names,
// settings, prompt history).
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import {
  api,
  errText,
  onAdapterDownload,
  onOpenPaths,
  onSessionEvent,
  type AgentInfo,
  type ClaudeRouteInfo,
  type EnvCheck,
  type SessionEvent,
} from "../api";
import { t, type Key } from "../i18n";
import { en } from "../i18n/en";
import type { Entry, ProjectInfo, UndoOutcome, Waiting } from "../types";
import { Session, type SavedSession, type SessionMode } from "./session.svelte";
import { paneTerms } from "./paneterm.svelte";
import { MODE_SWITCH_PRESSES, MODE_SWITCH_WAIT, titleTopic } from "../tui";
import { applyTheme, type Theme } from "../theme.svelte";
import { DEFAULT_BOTTOM, defaultDock, legacyDock, normalizeDock, type Dock } from "../dock";
import { updates } from "../updates.svelte";

export interface Settings {
  theme: Theme;
  fontSize: number;
  shell: string;
  agent: string;
  /** The command line for the "custom" agent. */
  agentCommand: string;
  /** Check GitHub Releases for updates in the background. */
  autoUpdate: boolean;
  showIgnored: boolean;
  notifications: boolean;
  /** Turn tracking in Claude panes: start them with the focuside plugin (--plugin-dir). On by
   * default. v0.2.0 stored null before its one-time question; null counts as on. */
  claudeHooks: boolean;
  /** Brief replies in Claude panes: the plugin also carries an output style that tells Claude to
   * answer in one line. Off by default (owner decision 2026-11-13). Needs turn tracking. */
  claudeBrief: boolean;
  /** Session tabs in the top bar: "top" shows the strip, "side" hides it. The Sessions view of
   * the side bar always lists them. */
  sessionTabs: SessionTabs;
  /** Agent ids whose chat mode the user confirmed once after a terms note (see CHAT_CONFIRM). */
  chatConfirmed: string[];
  /** Settings > Plan folder: "" for Claude's default (~/.claude/plans), else a folder relative
   * to the project root. Claude panes FocusIDE starts get it as `plansDirectory`; snapshots
   * skip it. */
  planFolder: string;
}

/** Agents that open in a pane by default and ask once before chat mode. Antigravity: Google's
 * terms on third-party clients are unclear for consumer logins, and FocusIDE cannot read which
 * login the user has (docs/agent-terms.md, Phase 4). */
export const CHAT_CONFIRM = new Set(["antigravity"]);

/** Agents that only run the provider the user set up in them; they share one terms line. */
const PROVIDER_AGENTS = new Set(["opencode", "goose", "qwen", "cline", "kimi", "vibe"]);

/** The one-line data and terms note for an agent, or "" when there is none. */
export function agentNote(id: string): string {
  if (PROVIDER_AGENTS.has(id)) return t("agentNote.provider");
  const key = `agentNote.${id}`;
  return key in en ? t(key as Key) : "";
}

export type SessionTabs = "top" | "side";

/** Panel places and sizes. The dock holds the Session panel, side bar, viewer, plan, prompts and terminal (see lib/dock.ts). */
export interface Layout {
  dock: Dock;
  /** Height of the bottom zone in pixels. */
  bottomHeight: number;
  /** The side bar view: the file tree or the session list. */
  sideView: "files" | "sessions";
  /** The Turns column beside an agent pane. */
  turnsVisible: boolean;
}

export function defaultLayout(): Layout {
  return { dock: defaultDock(), bottomHeight: DEFAULT_BOTTOM, sideView: "files", turnsVisible: true };
}

/**
 * A saved layout, including one saved before docking (filesSide, viewerSide, terminalHeight)
 * and one saved before the Session and Prompts panels (they go to their default places).
 */
function loadLayout(raw: Record<string, unknown> | undefined): Layout {
  const out = defaultLayout();
  if (!raw) return out;
  out.dock = normalizeDock(raw.dock) ?? legacyDock(raw);
  const h = raw.bottomHeight ?? raw.terminalHeight;
  if (typeof h === "number" && h >= 120) out.bottomHeight = h;
  if (raw.sideView === "sessions") out.sideView = "sessions";
  if (raw.turnsVisible === false) out.turnsVisible = false;
  return out;
}

interface Persisted {
  recent: { path: string; name: string; opened: number }[];
  open: string[];
  active: string | null;
  sessions: Record<string, SavedSession[]>;
  planFirst: boolean;
  history: string[];
  settings: Settings;
  filesVisible: boolean;
  layout: Layout;
  /** Editor tabs per project root (stores/editor.svelte.ts). */
  editor?: unknown;
}

const defaults: Persisted = {
  recent: [],
  open: [],
  active: null,
  sessions: {},
  planFirst: true,
  history: [],
  settings: { theme: "system", fontSize: 15, shell: "", agent: "claude", agentCommand: "", autoUpdate: true, showIgnored: false, notifications: true, claudeHooks: true, claudeBrief: false, sessionTabs: "top", chatConfirmed: [], planFolder: "" },
  filesVisible: true,
  layout: defaultLayout(),
};

export interface Project extends ProjectInfo {
  sessions: Session[];
  activeKey: string | null;
}

export type Overlay = null | "queue" | "sessions" | "palette" | "openFile" | "settings" | "model" | "new";

export type ViewerTarget =
  | null
  | { kind: "file"; projectId: string; path: string }
  | { kind: "diff"; projectId: string; turnId: string; paths: string[]; index: number; sessionKey: string };

/** The editor store, which owns the viewer's tabs. */
export interface ViewerHost {
  setViewer(v: ViewerTarget): void;
  /** Asks about unsaved files of a project before it closes. False: the user cancelled. */
  canCloseProject(projectId: string): Promise<boolean>;
}

class AppStore {
  ready = $state(false);
  env = $state<EnvCheck | null>(null);
  /** The claude-agent-acp download: null when idle, else the percent done. */
  adapterProgress = $state<number | null>(null);
  adapterError = $state<string | null>(null);
  /** A Claude session that could not start because the adapter was missing. It starts after the download. */
  private adapterWaiting = $state<{ resume?: SavedSession; agentId?: string } | null>(null);
  /** How Claude Code is signed in, as a route (agent::ClaudeRoute). Undefined until checked. */
  claudeRoute = $state<ClaudeRouteInfo | null | undefined>(undefined);
  private routeCheck: Promise<ClaudeRouteInfo | null> | null = null;
  projects = $state<Project[]>([]);
  activeId = $state<string | null>(null);
  overlay = $state<Overlay>(null);
  planFirst = $state(true);
  history = $state<string[]>([]);
  recent = $state<Persisted["recent"]>([]);
  savedSessions = $state<Record<string, SavedSession[]>>({});
  settings = $state<Settings>({ ...defaults.settings });
  filesVisible = $state(true);
  layout = $state<Layout>(defaultLayout());
  terminalVisible = $state(false);
  /** The Plan panel. Opens when a plan arrives; not kept between runs. */
  planVisible = $state(false);
  /** The Prompts panel (PromptsPanel.svelte). The dock shows it while this is true. */
  promptsVisible = $state(false);
  /** The prompt the Prompts panel opens and scrolls to (a click on a dot of the pane strip). */
  promptsFocus = $state<{ sessionKey: string; id: string | null; seq: number } | null>(null);
  /** The newest request to show a panel, for the dock: showPlan() and showPrompts() set it. */
  panelRequest = $state<{ panel: "plan" | "prompts"; seq: number } | null>(null);
  private panelSeq = 0;
  terminalCommand = $state<string | null>(null);
  focused = $state(true);
  toast = $state<{ text: string; tone: "muted" | "error" } | null>(null);
  /** Set while the terms note before chat mode is shown (CHAT_CONFIRM); called with the choice. */
  chatAsk = $state<{ agent: string; answer: (choice: "chat" | "pane" | null) => void } | null>(null);
  /** Undo dialog target. */
  undoTarget = $state<{ projectId: string; turnId: string; entryId: string; sessionKey: string } | null>(null);
  /** Last undo, for "Redo last undo" in the palette. */
  lastUndo = $state<{ projectId: string; turnId: string; entryId: string; sessionKey: string } | null>(null);
  /**
   * What the viewer shows: the active editor tab of the active project (file or diff). The
   * editor store (stores/editor.svelte.ts) holds the tabs and registers as `viewerHost`. Setting
   * a file opens it as a preview tab, a diff opens the diff tab, null hides the viewer.
   */
  get viewer(): ViewerTarget {
    return this.viewerShown;
  }
  set viewer(v: ViewerTarget) {
    if (this.viewerHost) this.viewerHost.setViewer(v);
    else this.viewerShown = v;
  }
  private viewerShown = $state<ViewerTarget>(null);
  viewerHost: ViewerHost | null = null;
  /** For the editor store: the tab the viewer shows now. */
  showViewer(v: ViewerTarget) {
    this.viewerShown = v;
  }
  /** Editor tabs per project (stores/editor.svelte.ts), as saved. Kept apart from the rest of the state. */
  editorSaved = $state<unknown>(null);

  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  private sessionsByKey = new Map<string, Session>();
  private dataTimers = new Map<string, ReturnType<typeof setTimeout>>();

  get active(): Project | undefined {
    return this.projects.find((p) => p.id === this.activeId);
  }

  get session(): Session | undefined {
    const p = this.active;
    return p?.sessions.find((s) => s.key === p.activeKey);
  }

  /** Every session that waits for the user, across projects. */
  get queue(): { project: Project; session: Session }[] {
    const out: { project: Project; session: Session }[] = [];
    for (const project of this.projects) {
      for (const session of project.sessions) if (session.waiting) out.push({ project, session });
    }
    return out;
  }

  projectWaiting(p: Project): boolean {
    return p.sessions.some((s) => s.waiting);
  }

  private inited = false;

  /** Loads state and listens to events. Runs once, even if the UI mounts again (hot reload). */
  async init() {
    if (this.inited) return;
    this.inited = true;
    try {
      const raw = (await api.uiStateLoad()) as Partial<Persisted> | null;
      const st: Persisted = { ...defaults, ...(raw ?? {}), settings: { ...defaults.settings, ...(raw?.settings ?? {}) } };
      this.recent = st.recent;
      this.planFirst = st.planFirst;
      this.history = st.history;
      this.savedSessions = st.sessions;
      this.settings = st.settings;
      // Turn tracking is on unless the user turned it off (v0.2.0 stored null before asking).
      if (typeof this.settings.claudeHooks !== "boolean") this.settings.claudeHooks = true;
      if (typeof this.settings.claudeBrief !== "boolean") this.settings.claudeBrief = false;
      if (this.settings.sessionTabs !== "side") this.settings.sessionTabs = "top";
      if (typeof this.settings.planFolder !== "string") this.settings.planFolder = "";
      // Snapshots skip the plan folder. A folder Rust refuses counts as Claude's default.
      if (this.settings.planFolder)
        await api.planFolderSet(this.settings.planFolder).catch(() => (this.settings.planFolder = ""));
      this.filesVisible = st.filesVisible;
      this.layout = loadLayout(raw?.layout as Record<string, unknown> | undefined);
      this.editorSaved = raw?.editor ?? null;
      applyTheme(this.settings.theme);
      document.documentElement.style.setProperty("--read-size", `${this.settings.fontSize}px`);
      this.env = await api.envCheck();
      this.loadAgents();
      if (this.env.claude) this.checkClaudeRoute();
      await onSessionEvent((e) => this.onEvent(e));
      await onOpenPaths((paths) => paths.forEach((p) => this.openPath(p)));
      const w = getCurrentWindow();
      await w.onFocusChanged(({ payload }) => {
        this.focused = payload;
        if (payload) this.clearSeen();
      });
      // Reopen the last projects in parallel, keeping their rail order.
      const opened = await Promise.all(st.open.map((path) => api.projectOpen(path).catch(() => null)));
      for (const info of opened) {
        if (info && !this.projects.some((p) => p.id === info.id)) this.projects.push({ ...info, sessions: [], activeKey: null });
      }
      const startup = await api.startupPaths();
      for (const p of startup) await this.openPath(p);
      if (!startup.length && st.active) {
        const p = this.projects.find((x) => x.root === st.active);
        if (p) this.activate(p.id);
      }
      if (!this.activeId && this.projects.length) this.activate(this.projects[0].id);
    } catch (e) {
      this.flash(errText(e), "error");
    }
    this.ready = true;
    updates.start(this.settings.autoUpdate);
  }

  resetLayout() {
    this.layout = defaultLayout();
    this.save();
  }

  flash(text: string, tone: "muted" | "error" = "muted") {
    this.toast = { text, tone };
    const mine = this.toast;
    setTimeout(() => {
      if (this.toast === mine) this.toast = null;
    }, tone === "error" ? 6000 : 2500);
  }

  save() {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.saveNow(), 400);
  }

  saveNow() {
    for (const p of this.projects) {
      const list = this.savedSessions[p.root] ?? [];
      for (const s of p.sessions) {
        const saved = s.saved();
        if (!saved) continue;
        const i = list.findIndex((x) => x.acpId === saved.acpId);
        if (i >= 0) list[i] = saved;
        else list.unshift(saved);
      }
      // Open sessions keep their tab order in the saved list. They swap places among
      // themselves, so the other entries stay where they are.
      const ids = p.sessions.map((s) => s.acpId).filter((id) => !!id);
      const slots = list.map((x, i) => (ids.includes(x.acpId) ? i : -1)).filter((i) => i >= 0);
      const ordered = slots.map((i) => list[i]).sort((a, b) => ids.indexOf(a.acpId) - ids.indexOf(b.acpId));
      slots.forEach((at, n) => (list[at] = ordered[n]));
      this.savedSessions[p.root] = list.slice(0, 100);
    }
    const st: Persisted = {
      recent: this.recent,
      open: this.projects.map((p) => p.root),
      active: this.active?.root ?? null,
      sessions: $state.snapshot(this.savedSessions),
      planFirst: this.planFirst,
      history: this.history.slice(-200),
      settings: $state.snapshot(this.settings),
      filesVisible: this.filesVisible,
      layout: $state.snapshot(this.layout),
      editor: $state.snapshot(this.editorSaved),
    };
    api.uiStateSave(st).catch(() => {});
  }

  updateTitle() {
    const p = this.active;
    const s = this.session;
    const n = this.queue.length;
    const parts = [p?.name, s?.name || (p ? t("session.untitled") : null), "FocusIDE"].filter(Boolean);
    let title = parts.join(" - ");
    if (n) title += ` (${t("bar.waiting", { count: n })})`;
    getCurrentWindow().setTitle(title).catch(() => {});
  }

  // Projects

  async pickFolder() {
    const path = await openDialog({ directory: true, multiple: false });
    if (typeof path === "string") await this.openPath(path);
  }

  async openPath(path: string, opts: { activate?: boolean; quiet?: boolean } = {}) {
    try {
      const info = await api.projectOpen(path);
      let p = this.projects.find((x) => x.id === info.id);
      if (!p) {
        this.projects.push({ ...info, sessions: [], activeKey: null });
        p = this.projects[this.projects.length - 1];
      }
      this.recent = [
        { path: info.root, name: info.name, opened: Date.now() },
        ...this.recent.filter((r) => r.path !== info.root),
      ].slice(0, 12);
      if (opts.activate !== false) this.activate(info.id);
      this.save();
    } catch (e) {
      if (!opts.quiet) this.flash(errText(e), "error");
    }
  }

  activate(id: string) {
    this.activeId = id;
    const p = this.active;
    if (p && !p.sessions.length) this.newSession();
    this.clearSeen();
    this.updateTitle();
    this.save();
  }

  async closeProject(id: string) {
    const p = this.projects.find((x) => x.id === id);
    if (!p) return;
    if (this.viewerHost && !(await this.viewerHost.canCloseProject(id))) return;
    this.saveNow();
    await api.projectClose(id).catch(() => {});
    for (const s of p.sessions) {
      this.sessionsByKey.delete(s.key);
      if (s.mode === "pane") paneTerms.dispose(s.key);
    }
    this.projects = this.projects.filter((x) => x.id !== id);
    if (this.activeId === id) this.activeId = this.projects[0]?.id ?? null;
    this.updateTitle();
    this.save();
  }

  // Sessions

  /** Known agents and whether each is installed. Loaded for Settings and the palette. */
  agents = $state<AgentInfo[]>([]);

  async loadAgents() {
    this.agents = await api.agentList().catch(() => []);
  }

  /** The adapter is missing and a Claude chat session needs it. A Claude pane does not need it. */
  get needsAdapter(): boolean {
    const claudeChat = this.settings.agent === "claude" && this.claudeRoute?.route !== "pane";
    return !!this.env && !this.env.adapter && (claudeChat || !!this.adapterWaiting);
  }

  /** Runs `claude auth status` once (or again with `force`) and keeps the route. */
  checkClaudeRoute(force = false): Promise<ClaudeRouteInfo | null> {
    if (force || !this.routeCheck) {
      this.routeCheck = api
        .claudeRoute()
        .catch(() => null)
        .then((r) => {
          this.claudeRoute = r;
          return r;
        });
    }
    return this.routeCheck;
  }

  /** Chat for an API key or a cloud provider; a Claude pane for a Claude plan login or no login. */
  private async claudeMode(): Promise<SessionMode> {
    if (!this.env?.claude) return "chat";
    const r = await this.checkClaudeRoute();
    return r?.route === "pane" ? "pane" : "chat";
  }

  /** Something the user has to install or download before a session can start. */
  get envProblem(): boolean {
    return !!this.env && (!this.env.claude || !this.env.git || this.needsAdapter);
  }

  /** Downloads the pinned adapter after the user chose Download, then starts the waiting session. */
  async downloadAdapter() {
    if (this.adapterProgress !== null) return;
    this.adapterProgress = 0;
    this.adapterError = null;
    const off = await onAdapterDownload(({ received, total }) => {
      this.adapterProgress = Math.min(100, Math.floor((received * 100) / Math.max(total, 1)));
    });
    try {
      await api.adapterDownload();
      this.env = await api.envCheck();
      const waiting = this.adapterWaiting;
      this.adapterWaiting = null;
      if (waiting && this.active) await this.newSession(waiting.resume, waiting.agentId);
    } catch (e) {
      this.adapterError = errText(e);
    } finally {
      off();
      this.adapterProgress = null;
    }
  }

  agentName(id: string): string {
    if (id === "custom") return t("agent.custom");
    if (id === "claude") return "Claude";
    return this.agents.find((a) => a.id === id)?.name ?? id;
  }

  /**
   * Starts a session. `mode` "pane" opens the agent's own TUI. Without a mode: a resumed
   * session keeps its mode, Claude follows its login (see claudeMode), other agents use chat.
   */
  async newSession(resume?: SavedSession, agentId?: string, mode?: SessionMode) {
    const p = this.active;
    if (!p) return;
    if (resume) {
      const live = p.sessions.find((s) => s.acpId === resume.acpId);
      if (live) {
        this.showSession(live.key);
        return;
      }
    }
    try {
      const agent = resume?.agent ?? agentId ?? this.settings.agent;
      let m: SessionMode = resume
        ? (resume.mode ?? "chat")
        : (mode ?? (agent === "claude" ? await this.claudeMode() : CHAT_CONFIRM.has(agent) ? "pane" : "chat"));
      if (m === "chat" && CHAT_CONFIRM.has(agent) && !this.settings.chatConfirmed.includes(agent)) {
        const choice = await this.confirmChat(agent);
        if (!choice) return;
        m = choice;
      }
      if (this.active !== p) return;
      // Claude panes: turn tracking (the focuside plugin) and Plan first (plan mode at start).
      const claudePane = m === "pane" && agent === "claude";
      const hooks = claudePane && this.settings.claudeHooks;
      const planFirst = claudePane && this.planFirst;
      const brief = hooks && this.settings.claudeBrief;
      const planFolder = claudePane && this.settings.planFolder ? this.settings.planFolder : null;
      const key =
        m === "pane"
          ? await api.sessionStart(p.id, null, agent, this.settings.agentCommand, "pane", hooks, planFirst, brief, planFolder)
          : await api.sessionStart(p.id, resume?.acpId ?? null, agent, this.settings.agentCommand);
      const s = new Session(key, p.id, p.root, resume);
      s.agent = agent;
      s.mode = m;
      s.status = "starting";
      s.replaying = !!resume && m === "chat";
      s.panePlan = planFirst;
      if (m === "pane" && !resume) {
        // A pane has no ACP session id. FocusIDE's own id lets the sessions list reopen it.
        s.acpId = `pane-${key}`;
      }
      if (!resume) {
        // A generic name until a prompt or the agent's title names the session.
        s.name = this.nextSessionName(p);
        s.generic = true;
      }
      s.onData = () => this.saveSessionData(s);
      if (resume) {
        const data = await api.sessionDataLoad(p.id, resume.acpId).catch(() => null);
        s.loadData(data);
      }
      this.sessionsByKey.set(key, s);
      p.sessions.push(s);
      p.activeKey = key;
      this.sessionsByKey.set(key, p.sessions[p.sessions.length - 1]);
      this.updateTitle();
    } catch (e) {
      const msg = errText(e);
      if (msg === "missing:agent") {
        const id = resume?.agent ?? agentId ?? this.settings.agent;
        if (!this.agents.length) await this.loadAgents();
        const info = this.agents.find((a) => a.id === id);
        this.flash(
          id === "custom" ? t("agent.customMissing") : t("agent.missing", { name: this.agentName(id), install: info?.install ?? "" }),
          "error",
        );
      } else if (msg.startsWith("missing:")) {
        if (msg === "missing:adapter") this.adapterWaiting = { resume, agentId };
        this.env = await api.envCheck();
      }
      else this.flash(msg, "error");
    }
  }

  /** "Session N": the lowest number no open session of the project uses. */
  private nextSessionName(p: Project): string {
    const used = new Set(p.sessions.map((s) => s.name));
    let n = p.sessions.length + 1;
    for (let i = 1; i <= n; i++) {
      if (!used.has(t("session.numbered", { n: i }))) {
        n = i;
        break;
      }
    }
    return t("session.numbered", { n });
  }

  /** Saves a session's plan versions and prompts to app data, shortly after they change. */
  private saveSessionData(s: Session) {
    const old = this.dataTimers.get(s.key);
    if (old) clearTimeout(old);
    this.dataTimers.set(
      s.key,
      setTimeout(() => {
        this.dataTimers.delete(s.key);
        const data = s.data();
        if (s.acpId && data) api.sessionDataSave(s.projectId, s.acpId, data).catch(() => {});
      }, 500),
    );
  }

  /** A terminal title from a pane's TUI (OSC 0 or 2). A topic becomes the tab name unless the
   * user renamed the session. Titles that are not a topic (a program path, the agent's name)
   * are ignored without a message. */
  paneTitle(s: Session, raw: string) {
    const topic = titleTopic(raw, [this.agentName(s.agent), s.agent]);
    if (topic && s.setTitle(topic)) {
      this.updateTitle();
      this.save();
    }
  }

  /** Shows the Plan panel and brings it to the front of its dock group. */
  showPlan() {
    this.planVisible = true;
    this.panelRequest = { panel: "plan", seq: ++this.panelSeq };
    import("./dock.svelte").then(({ dock }) => dock.show("plan")).catch(() => {});
  }

  /** Shows the Prompts panel, opened at one prompt when `id` is given. The dock shows the panel
   * while `promptsVisible` is true (panel id "prompts"). */
  showPrompts(sessionKey: string, id: string | null = null) {
    this.promptsVisible = true;
    const seq = ++this.panelSeq;
    this.promptsFocus = { sessionKey, id, seq };
    this.panelRequest = { panel: "prompts", seq };
    import("./dock.svelte")
      .then(({ dock }) => dock.show("prompts"))
      .catch(() => {});
  }

  /** Settings > Plan folder. "" is Claude's default. Returns false for a folder outside the project. */
  async setPlanFolder(folder: string): Promise<boolean> {
    try {
      const clean = await api.planFolderSet(folder.trim() || null);
      this.settings.planFolder = clean ?? "";
      this.save();
      return true;
    } catch {
      return false;
    }
  }

  /** Asks once before chat mode with an agent in CHAT_CONFIRM. Null: the user cancelled. */
  private confirmChat(agent: string): Promise<"chat" | "pane" | null> {
    return new Promise((resolve) => {
      this.chatAsk = {
        agent,
        answer: (choice) => {
          this.chatAsk = null;
          if (choice === "chat") {
            this.settings.chatConfirmed = [...this.settings.chatConfirmed, agent];
            this.save();
          }
          resolve(choice);
        },
      };
    });
  }

  showSession(key: string) {
    for (const p of this.projects) {
      if (p.sessions.some((s) => s.key === key)) {
        p.activeKey = key;
        this.activeId = p.id;
      }
    }
    this.clearSeen();
    this.updateTitle();
    this.save();
  }

  /** Ctrl+Tab and Ctrl+Shift+Tab: the next or previous session of the active project. */
  cycleSession(step: 1 | -1) {
    const p = this.active;
    if (!p || p.sessions.length < 2) return;
    const i = p.sessions.findIndex((s) => s.key === p.activeKey);
    const next = p.sessions[(i + step + p.sessions.length) % p.sessions.length];
    this.showSession(next.key);
  }

  /** Mod+1 to Mod+9: the session at that place in the tabs. */
  showSessionAt(index: number) {
    const s = this.active?.sessions[index];
    if (s) this.showSession(s.key);
  }

  /** Moves a session tab to another place in the tabs of its project. */
  moveSession(key: string, index: number) {
    const p = this.projects.find((x) => x.sessions.some((s) => s.key === key));
    if (!p) return;
    const from = p.sessions.findIndex((s) => s.key === key);
    const to = Math.max(0, Math.min(p.sessions.length - 1, index));
    if (from === to) return;
    const list = [...p.sessions];
    const [s] = list.splice(from, 1);
    list.splice(to, 0, s);
    p.sessions = list;
    this.save();
  }

  /** Ctrl+Shift+Page Up and Page Down: moves the active tab one place. */
  moveActiveSession(step: 1 | -1) {
    const p = this.active;
    if (!p?.activeKey) return;
    const i = p.sessions.findIndex((s) => s.key === p.activeKey);
    this.moveSession(p.activeKey, i + step);
  }

  async closeSession(key: string) {
    const p = this.projects.find((x) => x.sessions.some((s) => s.key === key));
    if (!p) return;
    this.saveNow();
    const closing = this.sessionsByKey.get(key);
    const pending = this.dataTimers.get(key);
    if (closing && pending) {
      // Write the plan versions and prompts now, not after the session is gone.
      clearTimeout(pending);
      this.dataTimers.delete(key);
      const data = closing.data();
      if (closing.acpId && data) await api.sessionDataSave(closing.projectId, closing.acpId, data).catch(() => {});
    }
    await api.sessionClose(key).catch(() => {});
    paneTerms.dispose(key);
    p.sessions = p.sessions.filter((s) => s.key !== key);
    this.sessionsByKey.delete(key);
    if (p.activeKey === key) p.activeKey = p.sessions[p.sessions.length - 1]?.key ?? null;
    this.updateTitle();
  }

  renameSession(s: Session, name: string) {
    const n = name.trim();
    if (!n) return;
    s.name = n;
    s.renamed = true;
    this.updateTitle();
    this.save();
  }

  /** "Done" and "failed" clear once the user looks at the session. */
  clearSeen() {
    const s = this.session;
    if (s && this.focused && (s.waiting === "done" || s.waiting === "failed")) s.waiting = null;
  }

  sessionFor(key: string): Session | undefined {
    for (const p of this.projects) {
      const s = p.sessions.find((x) => x.key === key);
      if (s) return s;
    }
    return undefined;
  }

  private onEvent(e: SessionEvent) {
    if (e.kind === "claude_route") this.claudeRoute = { access: e.access, route: e.route };
    const s = this.sessionFor(e.key);
    if (!s) return;
    // A prompt that was not tracked: name the session that held the project.
    if (e.kind === "turn_untracked" && typeof e.holder === "string") e.holderName = this.sessionFor(e.holder)?.name ?? "";
    const versions = s.planVersions.length;
    const named = s.name;
    const w = s.handle(e);
    // A new plan version opens the Plan panel at that version.
    if (s.planVersions.length > versions && !s.replaying && this.session === s) this.showPlan();
    if (s.name !== named) this.save();
    if (e.kind === "ready") {
      s.replaying = false;
      this.save();
    }
    if (e.kind === "turn_ended" || e.kind === "pane_started") this.save();
    if (w !== "none" && w) this.notifyWaiting(s, w);
    this.clearSeen();
    this.updateTitle();
  }

  private async notifyWaiting(s: Session, w: Exclude<Waiting, null>) {
    if (!this.settings.notifications) return;
    const visible = this.focused && this.session?.key === s.key;
    const sameProject = this.focused && this.activeId === s.projectId;
    // System notification only when the session is in a project the user is not looking at.
    if (visible || sameProject) return;
    try {
      let ok = await isPermissionGranted();
      if (!ok) ok = (await requestPermission()) === "granted";
      if (!ok) return;
      const p = this.projects.find((x) => x.id === s.projectId);
      const body = w === "plan" ? t("notify.plan") : w === "permission" ? t("notify.permission") : t("notify.done");
      sendNotification({ title: t("notify.waiting", { project: p?.name ?? "", session: s.name }), body });
    } catch {
      // Notifications are optional.
    }
  }

  // Turn actions

  /** Sends the message box text. Handles a waiting plan (revise) and Plan first. */
  async send(text: string, extra: unknown[] = [], files: string[] = []) {
    const s = this.session;
    if (!s) return;
    const trimmed = text.trim();
    if (!trimmed && !extra.length) return;
    this.history = [...this.history.filter((h) => h !== trimmed), trimmed].slice(-200);
    const plan = s.waitingPlan;
    if (plan?.requestId && trimmed) {
      s.addUser(trimmed);
      try {
        await api.sessionRevisePlan(s.key, plan.requestId, trimmed);
      } catch (e) {
        this.flash(errText(e), "error");
      }
      this.save();
      return;
    }
    const blocks: unknown[] = [];
    if (trimmed) blocks.push({ type: "text", text: trimmed });
    blocks.push(...extra);
    const images = extra.filter((b: any) => b.type === "image").length;
    s.addUser(trimmed, images, files);
    try {
      await api.sessionPrompt(s.key, blocks, this.planFirst && s.planMode);
    } catch (e) {
      this.promptError(s, errText(e));
    }
    this.save();
  }

  private promptError(s: Session, msg: string) {
    if (msg === "otherRunning") s.note(t("session.otherRunning"), "warn");
    else if (msg === "busy") s.note(t("conv.working"), "muted");
    else if (msg === "signin" || msg === "claude_route") s.status = "claudeAccess";
    else s.note(t("conv.error", { error: msg }), "error");
  }

  async runPlan(s: Session, entry: Extract<Entry, { t: "plan" }>) {
    if (!entry.requestId || entry.state !== "waiting") return;
    try {
      await api.sessionRunPlan(s.key, entry.requestId);
    } catch (e) {
      this.promptError(s, errText(e));
    }
  }

  async discardPlan(s: Session, entry: Extract<Entry, { t: "plan" }>) {
    if (!entry.requestId || entry.state !== "waiting") return;
    try {
      await api.sessionDiscardPlan(s.key, entry.requestId);
    } catch (e) {
      this.flash(errText(e), "error");
    }
  }

  async continuePlan(s: Session, entry: Extract<Entry, { t: "plan" }>) {
    if (!entry.turnId) return;
    try {
      await api.sessionPrompt(s.key, [{ type: "text", text: t("plan.continuePrompt") }], false, entry.turnId);
    } catch (e) {
      this.promptError(s, errText(e));
    }
  }

  async stop(s: Session | undefined = this.session) {
    // A pane has no Stop: the user stops the agent in its own TUI.
    if (!s || !s.running || s.mode === "pane") return;
    s.stopping = true;
    await api.sessionCancel(s.key).catch((e) => this.flash(errText(e), "error"));
  }

  async answer(s: Session, entry: Extract<Entry, { t: "permission" }>, optionId: string) {
    try {
      await api.sessionAnswer(s.key, entry.requestId, optionId);
      entry.state = "answered";
      entry.answer = entry.options.find((o) => o.optionId === optionId)?.name;
      if (s.waiting === "permission") s.waiting = null;
    } catch (e) {
      this.flash(errText(e), "error");
    }
  }

  /** Is another session of this project running a turn? */
  otherRunning(s: Session): boolean {
    const p = this.projects.find((x) => x.id === s.projectId);
    return !!p?.sessions.some((x) => x.key !== s.key && x.claims);
  }

  /** Opens the undo dialog for a turn: a plan or changes entry, or a prompt of the Prompts panel. */
  askUndo(s: Session, entry: { id: string; turnId: string | null }) {
    if (!entry.turnId) return;
    this.undoTarget = { projectId: s.projectId, turnId: entry.turnId, entryId: entry.id, sessionKey: s.key };
  }

  /** Sets the undo state of every entry and prompt of a turn. */
  private markUndo(target: { sessionKey: string; turnId: string }, undo: { outcome: UndoOutcome } | null) {
    const s = this.sessionFor(target.sessionKey);
    if (!s) return;
    for (const e of s.entries) if ((e.t === "plan" || e.t === "changes") && e.turnId === target.turnId) e.undo = undo;
    for (const p of s.prompts) if (p.turnId === target.turnId) p.undo = undo;
    if (s.prompts.some((p) => p.turnId === target.turnId)) this.saveSessionData(s);
  }

  entryOf(target: { entryId: string; sessionKey: string }) {
    const s = this.sessionFor(target.sessionKey);
    const e = s?.entries.find((x) => x.id === target.entryId);
    return { s, e };
  }

  applyUndo(target: { entryId: string; sessionKey: string; projectId: string; turnId: string }, outcome: UndoOutcome) {
    const { e } = this.entryOf(target);
    if (e && (e.t === "plan" || e.t === "changes")) e.undo = { outcome };
    this.markUndo(target, { outcome });
    this.lastUndo = target;
    // Chat sessions tell the agent in the next prompt. A pane gets nothing typed in, so the
    // user is told instead.
    const s = this.sessionFor(target.sessionKey);
    if (s?.mode === "pane") s.note(t("pane.undoNote", { agent: this.agentName(s.agent) }), "warn");
  }

  /**
   * The plan mode switch of a Claude pane: turns plan mode on or off. Claude has one key for
   * modes (Shift+Tab moves to the next one), so the switch presses it, waits for the status
   * line to change, reads the mode again, and stops at the target (at most
   * MODE_SWITCH_PRESSES presses). On: plan mode. Off: the mode before plan mode, else manual.
   * Keys go only on this click and only while no prompt runs (pane.rs refuses them otherwise).
   * When the mode cannot be read, nothing is sent.
   */
  async togglePlanMode(s: Session) {
    if (s.mode !== "pane" || s.agent !== "claude" || s.modeSwitching) return;
    const term = paneTerms.find(s.key);
    let mode = term?.shownMode() ?? null;
    if (!term || !mode) return;
    const on = mode !== "plan";
    if (on) s.modeBeforePlan = mode;
    const target = on ? "plan" : s.modeBeforePlan && s.modeBeforePlan !== "plan" ? s.modeBeforePlan : "default";
    s.modeSwitching = true;
    try {
      for (let i = 0; i < MODE_SWITCH_PRESSES && mode !== target; i++) {
        await api.paneModeKey(s.key);
        const next = await term.waitForMode(mode, MODE_SWITCH_WAIT);
        if (!next) break;
        mode = next;
        // Turning off: any mode that is not plan is better than another round of the cycle.
        if (!on && mode !== "plan" && i === MODE_SWITCH_PRESSES - 1) break;
      }
    } catch (e) {
      const msg = errText(e);
      this.flash(msg === "busy" ? t("pane.modeBusy") : msg === "closed" ? t("pane.exitedShort") : msg, "error");
    } finally {
      s.modeSwitching = false;
      s.screenMode = term.shownMode();
      term.focus();
    }
  }

  /** A chat session that was routed (Claude plan login): replace it with a Claude pane. */
  async openPaneInstead(s: Session) {
    if (s.access) this.claudeRoute = { access: s.access, route: "pane" };
    const agent = s.agent;
    const p = this.projects.find((x) => x.id === s.projectId);
    if (p && this.activeId !== p.id) this.activate(p.id);
    await this.closeSession(s.key);
    await this.newSession(undefined, agent, "pane");
  }

  async redo() {
    const target = this.lastUndo;
    if (!target) return;
    try {
      const out = await api.redo(target.projectId, target.turnId);
      const { e } = this.entryOf(target);
      if (e && (e.t === "plan" || e.t === "changes")) e.undo = null;
      this.markUndo(target, null);
      this.lastUndo = null;
      this.flash(out.exact ? t("undo.redone") : t("changes.undonePartial"));
    } catch (e) {
      this.flash(errText(e), "error");
    }
  }

  async resumeAgent(s: Session) {
    try {
      s.status = "starting";
      if (s.agent === "claude") this.checkClaudeRoute(true);
      await api.sessionResume(s.key);
    } catch (e) {
      const msg = errText(e);
      if (msg === "claude_route" || msg === "signin") s.status = "claudeAccess";
      else {
        s.status = "failed";
        s.error = msg;
      }
    }
  }

  /** Opens the terminal drawer and runs a command in its shell (install commands). Never used
   * for agent panes: FocusIDE does not type into an agent's TUI. */
  runInTerminal(command: string) {
    this.terminalVisible = true;
    this.terminalCommand = command;
  }

  setPlanFirst(on: boolean) {
    this.planFirst = on;
    this.save();
  }
}

export const app = new AppStore();
