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
import { t } from "../i18n";
import type { Entry, ProjectInfo, UndoOutcome, Waiting } from "../types";
import { Session, type SavedSession, type SessionMode } from "./session.svelte";
import { paneTerms } from "./paneterm.svelte";
import { applyTheme, type Theme } from "../theme.svelte";
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
}

export type Side = "left" | "right";

/** Panel sizes and sides. Widths in pixels. */
export interface Layout {
  filesWidth: number;
  viewerWidth: number;
  terminalHeight: number;
  filesSide: Side;
  viewerSide: Side;
}

export const defaultLayout: Layout = { filesWidth: 248, viewerWidth: 640, terminalHeight: 260, filesSide: "left", viewerSide: "right" };

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
}

const defaults: Persisted = {
  recent: [],
  open: [],
  active: null,
  sessions: {},
  planFirst: true,
  history: [],
  settings: { theme: "system", fontSize: 15, shell: "", agent: "claude", agentCommand: "", autoUpdate: true, showIgnored: false, notifications: true },
  filesVisible: true,
  layout: { ...defaultLayout },
};

export interface Project extends ProjectInfo {
  sessions: Session[];
  activeKey: string | null;
}

export type Overlay = null | "queue" | "sessions" | "palette" | "openFile" | "search" | "settings" | "model" | "new";

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
  layout = $state<Layout>({ ...defaultLayout });
  terminalVisible = $state(false);
  terminalCommand = $state<string | null>(null);
  focused = $state(true);
  toast = $state<{ text: string; tone: "muted" | "error" } | null>(null);
  /** Undo dialog target. */
  undoTarget = $state<{ projectId: string; turnId: string; entryId: string; sessionKey: string } | null>(null);
  /** Last undo, for "Redo last undo" in the palette. */
  lastUndo = $state<{ projectId: string; turnId: string; entryId: string; sessionKey: string } | null>(null);
  /** Viewer target (file or diff), set by M2 parts. */
  viewer = $state<
    | null
    | { kind: "file"; projectId: string; path: string }
    | { kind: "diff"; projectId: string; turnId: string; paths: string[]; index: number; sessionKey: string }
  >(null);

  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  private sessionsByKey = new Map<string, Session>();

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
      this.filesVisible = st.filesVisible;
      this.layout = { ...defaultLayout, ...(raw?.layout ?? {}) };
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

  /** Moves a panel to the other side of the conversation. */
  movePanel(panel: "files" | "viewer") {
    const key = panel === "files" ? "filesSide" : "viewerSide";
    this.layout[key] = this.layout[key] === "left" ? "right" : "left";
    this.save();
  }

  resetLayout() {
    this.layout = { ...defaultLayout };
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
      const m: SessionMode = resume ? (resume.mode ?? "chat") : (mode ?? (agent === "claude" ? await this.claudeMode() : "chat"));
      if (this.active !== p) return;
      const key =
        m === "pane"
          ? await api.sessionStart(p.id, null, agent, this.settings.agentCommand, "pane")
          : await api.sessionStart(p.id, resume?.acpId ?? null, agent, this.settings.agentCommand);
      const s = new Session(key, p.id, p.root, resume);
      s.agent = agent;
      s.mode = m;
      s.status = "starting";
      s.replaying = !!resume && m === "chat";
      if (m === "pane" && !resume) {
        // A pane has no ACP session id. FocusIDE's own id lets the sessions list reopen it.
        s.acpId = `pane-${key}`;
        s.name = t("pane.name", { agent: this.agentName(agent) });
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

  async closeSession(key: string) {
    const p = this.projects.find((x) => x.sessions.some((s) => s.key === key));
    if (!p) return;
    this.saveNow();
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

  private sessionFor(key: string): Session | undefined {
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
    const w = s.handle(e);
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

  askUndo(s: Session, entry: Extract<Entry, { t: "plan" }> | Extract<Entry, { t: "changes" }>) {
    if (!entry.turnId) return;
    this.undoTarget = { projectId: s.projectId, turnId: entry.turnId, entryId: entry.id, sessionKey: s.key };
  }

  entryOf(target: { entryId: string; sessionKey: string }) {
    const s = this.sessionFor(target.sessionKey);
    const e = s?.entries.find((x) => x.id === target.entryId);
    return { s, e };
  }

  applyUndo(target: { entryId: string; sessionKey: string; projectId: string; turnId: string }, outcome: UndoOutcome) {
    const { e } = this.entryOf(target);
    if (e && (e.t === "plan" || e.t === "changes")) e.undo = { outcome };
    this.lastUndo = target;
    // Chat sessions tell the agent in the next prompt. A pane gets nothing typed in, so the
    // user is told instead.
    const s = this.sessionFor(target.sessionKey);
    if (s?.mode === "pane") s.note(t("pane.undoNote", { agent: this.agentName(s.agent) }), "warn");
  }

  /** Checkpoint in a pane: the turn ends (changes card) and the next one starts. */
  async checkpoint(s: Session) {
    if (s.mode !== "pane") return;
    try {
      await api.sessionCheckpoint(s.key);
    } catch (e) {
      this.flash(errText(e), "error");
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
