// One agent session in the UI: its conversation entries, plan cards, waiting state and the
// live scope guard. Fed by `session` events from Rust.
import type { SessionEvent } from "../api";
import { t } from "../i18n";
import type {
  AgentCommand,
  ConfigOption,
  Entry,
  OutOfPlan,
  ParsedPlan,
  PermissionOption,
  PlanVersion,
  PromptTurn,
  TurnResult,
  Waiting,
} from "../types";

export type Status = "idle" | "starting" | "ready" | "stopped" | "failed" | "signin" | "claudeAccess";

let seq = 0;
const nid = () => `e${++seq}`;

/** Relative path with forward slashes when `abs` is inside `root`. */
export function relTo(root: string, abs: string): string | null {
  const norm = (s: string) => s.replace(/\\/g, "/").replace(/\/+$/, "");
  const r = norm(root);
  const p = norm(abs);
  const ci = /^[a-zA-Z]:/.test(r);
  const rc = ci ? r.toLowerCase() : r;
  const pc = ci ? p.toLowerCase() : p;
  if (pc === rc) return "";
  if (pc.startsWith(rc + "/")) return p.slice(r.length + 1);
  return null;
}

/** A tool title for display: project paths relative, forward slashes, no leading cd into the root. */
export function cleanTitle(root: string, title: string, paths: string[] = []): string {
  let s = title;
  for (const p of paths) {
    const rel = relTo(root, p);
    if (rel) s = s.split(p).join(rel);
  }
  for (const m of s.match(/[A-Za-z]:[\\/][^\s"']+|\/[^\s"']+/g) ?? []) {
    const rel = relTo(root, m);
    if (rel) s = s.split(m).join(rel);
  }
  s = s.replace(/\\/g, "/");
  const cd = s.match(/^cd\s+("[^"]+"|\S+)\s*&&\s*/);
  if (cd && relTo(root, cd[1].replace(/"/g, "")) === "") s = s.slice(cd[0].length);
  return s;
}

/** Last sentence of text, short, for "Not in plan: <reason>". */
function reasonFrom(text: string): string {
  const clean = text.replace(/[`*_#>]/g, "").trim();
  if (!clean) return "";
  const parts = clean.split(/(?<=[.!?])\s+|\n+/).filter((s) => s.trim());
  const last = (parts[parts.length - 1] ?? "").trim().replace(/[.:]+$/, "");
  return last.length > 70 ? last.slice(0, 67).trimEnd() + "..." : last;
}

export interface SavedSession {
  acpId: string;
  name: string;
  renamed: boolean;
  created: number;
  updated: number;
  /** Agent id ("claude", "codex", ...). Missing in sessions saved before agents could be chosen. */
  agent?: string;
  /** "pane" for the agent's own TUI. Missing means chat. For a pane, `acpId` is FocusIDE's own id. */
  mode?: SessionMode;
  /** The name came from the agent (a terminal title or an ACP session title). */
  titled?: boolean;
  /** The name is still "Session N". */
  generic?: boolean;
}

/** What a session keeps in app data between runs (session_data.rs). */
export interface SessionData {
  v: 1;
  planVersions: PlanVersion[];
  /** Pane prompts. Chat prompts come from the conversation. */
  prompts: PromptTurn[];
}

const MAX_VERSIONS = 100;
const MAX_PROMPTS = 500;

/** Chat: FocusIDE's own UI over ACP. Pane: the agent's own TUI in a terminal in the center. */
export type SessionMode = "chat" | "pane";

/**
 * Turn tracking in a pane. "otherRunning": another session held the project, so the turn is not
 * tracked. "waiting": hooks start the next turn when the user sends a prompt.
 */
export type Tracking = "off" | "on" | "otherRunning" | "error" | "waiting";

export class Session {
  key: string;
  projectId: string;
  root: string;
  acpId = $state<string | null>(null);
  agent = "claude";
  /** The agent has a plan mode, so "Plan first" can work. */
  planMode = $state(true);
  /** Why Claude did not start in chat mode: a Claude plan login, or not signed in at all. */
  access = $state<"subscription" | "signedOut" | null>(null);
  /** Where that login can run instead (agent::ClaudeRoute): a Claude pane. */
  route = $state<"pane" | null>(null);
  mode = $state<SessionMode>("chat");
  /** Pane: the agent's TUI runs. */
  paneAlive = $state(false);
  tracking = $state<Tracking>("off");
  trackingError = $state<string | null>(null);
  /** Pane: started with the focuside plugin, so hooks should report turns. */
  hooksExpected = $state(false);
  /** Pane: started in the agent's plan mode (Plan first, Claude only). */
  panePlan = $state(false);
  /** Pane: a hook event arrived. Turns start on a prompt and end when the agent stops. */
  hooksLive = $state(false);
  /** Pane: a prompt runs (hooks: between UserPromptSubmit and Stop). */
  paneWorking = $state(false);
  /** Pane (Claude): the permission mode, from the last hook event or the TUI's status line.
   * Null when unknown. */
  paneMode = $state<string | null>(null);
  /** Pane (Claude): the mode the TUI's status line shows now. Null when it cannot be read; the
   * plan mode switch is then off. */
  screenMode = $state<string | null>(null);
  /** The plan mode switch is pressing keys. */
  modeSwitching = $state(false);
  /** The mode before the switch turned plan mode on; turning it off goes back to it. */
  modeBeforePlan: string | null = null;
  /** Plan history: every plan and todo list version (kept in app data). */
  planVersions = $state<PlanVersion[]>([]);
  /** Pane prompts with their answers (kept in app data). */
  prompts = $state<PromptTurn[]>([]);
  /** Pane: the name of the session that held the project when the last prompt was not tracked. */
  untrackedBy = $state<string | null>(null);
  /** Called when the data in `data()` changed, so the app can save it. */
  onData: (() => void) | null = null;
  /** Pane: the agent's todo list (TodoWrite or the Task tools, from hooks). */
  paneTodos = $state<{ content: string; status: string }[]>([]);
  name = $state("");
  /** The user named the session (rename). Then nothing else replaces the name. */
  renamed = $state(false);
  /** The name came from the agent (terminal title or ACP session title). */
  titled = $state(false);
  /** The name is still the placeholder "Session N". */
  generic = $state(false);
  private fromSaved = false;
  status = $state<Status>("idle");
  error = $state<string | null>(null);
  entries = $state<Entry[]>([]);
  /** A turn is running (prompt sent, no response yet). */
  running = $state(false);
  turnId = $state<string | null>(null);
  stopping = $state(false);
  waiting = $state<Waiting>(null);
  thinking = $state(false);
  todo = $state<{ content: string; status: string }[]>([]);
  usage = $state<{ used: number; size: number } | null>(null);
  configOptions = $state<ConfigOption[]>([]);
  commands = $state<AgentCommand[]>([]);
  images = $state(false);
  draft = $state("");
  created = Date.now();
  updated = $state(Date.now());

  private toolKinds = new Map<string, string>();
  /** Text the agent wrote since the last tool call, for amber reasons. */
  private recentText = "";
  /** Set while the session's history is replayed after a resume. */
  replaying = false;

  constructor(key: string, projectId: string, root: string, saved?: SavedSession) {
    this.key = key;
    this.projectId = projectId;
    this.root = root;
    if (saved) {
      this.acpId = saved.acpId;
      this.name = saved.name;
      this.renamed = saved.renamed;
      this.created = saved.created;
      this.updated = saved.updated;
      this.agent = saved.agent ?? "claude";
      this.mode = saved.mode ?? "chat";
      this.titled = !!saved.titled;
      this.generic = !!saved.generic;
      this.fromSaved = true;
    }
  }

  /** A new name from a prompt: only while no title came from the agent and the user did not
   * rename the session. */
  private nameFromPrompt(text: string) {
    if (this.renamed || this.titled || !text.trim()) return;
    this.name = nameFrom(text);
    this.generic = false;
  }

  /** A topic from the agent (terminal title or ACP title). Never replaces the user's name. */
  setTitle(topic: string): boolean {
    if (this.renamed || !topic || topic === this.name) return false;
    this.name = topic;
    this.titled = true;
    this.generic = false;
    return true;
  }

  /** The data kept in app data, or null when there is nothing to keep. */
  data(): SessionData | null {
    if (!this.planVersions.length && !this.prompts.length) return null;
    return { v: 1, planVersions: $state.snapshot(this.planVersions), prompts: $state.snapshot(this.prompts) };
  }

  /** Data saved in an earlier run. Unknown shapes are ignored. */
  loadData(d: unknown) {
    const v = d as Partial<SessionData> | null;
    if (!v || typeof v !== "object") return;
    if (Array.isArray(v.planVersions) && !this.planVersions.length) this.planVersions = v.planVersions.slice(-MAX_VERSIONS);
    if (Array.isArray(v.prompts) && !this.prompts.length) this.prompts = v.prompts.slice(-MAX_PROMPTS);
  }

  private changed() {
    this.onData?.();
  }

  /** Adds a plan version unless it is the same text as the newest plan. Returns true for a
   * new version. */
  addPlanVersion(text: string, approved: boolean | null, turnId: string | null): boolean {
    if (!text.trim()) return false;
    const last = this.planVersions.findLast((x) => x.source === "plan");
    if (last && last.text === text) {
      if (approved && !last.approved) {
        last.approved = true;
        this.changed();
      }
      return false;
    }
    this.pushVersion({ n: 0, at: Date.now(), source: "plan", text, approved, turnId });
    return true;
  }

  /** A todo list: a new version when its items changed, else the statuses update in place. */
  addTodoVersion(todos: { content: string; status: string }[]): boolean {
    if (!todos.length) return false;
    const text = todos.map((x) => `- ${x.content}`).join("\n");
    const last = this.planVersions.findLast((x) => x.source === "todos");
    if (last && last.text === text) {
      last.todos = todos.map((x) => ({ ...x }));
      this.changed();
      return false;
    }
    this.pushVersion({ n: 0, at: Date.now(), source: "todos", text, approved: null, turnId: this.turnId, todos: todos.map((x) => ({ ...x })) });
    return true;
  }

  private pushVersion(v: PlanVersion) {
    v.n = (this.planVersions[this.planVersions.length - 1]?.n ?? 0) + 1;
    this.planVersions.push(v);
    if (this.planVersions.length > MAX_VERSIONS) this.planVersions.splice(0, this.planVersions.length - MAX_VERSIONS);
    this.changed();
  }

  /** The prompts for the Prompts panel, oldest first. Panes: from hooks. Chat: from the
   * conversation (each user message with the agent text and the changes after it). */
  get promptList(): PromptTurn[] {
    if (this.mode === "pane") return this.prompts;
    const out: PromptTurn[] = [];
    let cur: PromptTurn | null = null;
    for (const e of this.entries) {
      if (e.t === "user") {
        cur = { id: e.id, at: e.at ?? null, prompt: e.text, answer: null, turnId: null, result: null, undo: null };
        out.push(cur);
      } else if (!cur) continue;
      else if (e.t === "agent") cur.answer = (cur.answer ? cur.answer + "\n\n" : "") + e.text;
      else if ((e.t === "plan" || e.t === "changes") && e.result) {
        cur.turnId = e.turnId;
        cur.result = e.result;
        cur.undo = e.undo;
      }
    }
    return out;
  }

  get model(): ConfigOption | undefined {
    return this.configOptions.find((o) => o.id === "model" || o.category === "model");
  }

  /** Working on a prompt: a chat turn, or a pane prompt that hooks reported. For the tabs. */
  get busy(): boolean {
    return this.mode === "pane" ? this.paneWorking : this.running;
  }

  /** True while this session holds its project for a turn that can change files. */
  get claims(): boolean {
    if (this.mode === "pane") return this.tracking === "on";
    if (!this.running) return false;
    const p = this.entries.findLast((x) => x.t === "plan" && x.turnId === this.turnId);
    return !(p && p.t === "plan" && (p.state === "planning" || p.state === "waiting"));
  }

  /** The plan card that waits for the user, if any. */
  get waitingPlan(): Extract<Entry, { t: "plan" }> | undefined {
    for (let i = this.entries.length - 1; i >= 0; i--) {
      const e = this.entries[i];
      if (e.t === "plan" && e.state === "waiting") return e;
    }
    return undefined;
  }

  /** The plan card of the current or last turn. */
  lastPlan(): Extract<Entry, { t: "plan" }> | undefined {
    for (let i = this.entries.length - 1; i >= 0; i--) {
      const e = this.entries[i];
      if (e.t === "plan") return e;
      if (e.t === "user") return undefined;
    }
    return undefined;
  }

  saved(): SavedSession | null {
    // Sessions nobody used are not worth listing.
    if (!this.acpId || !this.name) return null;
    if (this.generic && !this.fromSaved && !this.renamed) return null;
    return {
      acpId: this.acpId,
      name: this.name,
      renamed: this.renamed,
      created: this.created,
      updated: this.updated,
      agent: this.agent,
      mode: this.mode,
      titled: this.titled,
      generic: this.generic,
    };
  }

  addUser(text: string, images = 0, files: string[] = []) {
    this.entries.push({ t: "user", id: nid(), text, images, files, at: Date.now() });
    this.nameFromPrompt(text);
    this.updated = Date.now();
  }

  note(text: string, tone: "muted" | "error" | "warn" = "muted") {
    this.entries.push({ t: "note", id: nid(), text, tone });
  }

  /** Adds an entry. While a plan is being made, its card stays last, so new lines go above it. */
  private add(entry: Entry) {
    const n = this.entries.length;
    const last = this.entries[n - 1];
    if (last && last.t === "plan" && last.state === "planning") this.entries.splice(n - 1, 0, entry);
    else this.entries.push(entry);
  }

  /** The newest entry that is not a planning card. */
  private tail(): Entry | undefined {
    const n = this.entries.length;
    const last = this.entries[n - 1];
    if (last && last.t === "plan" && last.state === "planning") return this.entries[n - 2];
    return last;
  }

  private appendAgent(text: string) {
    const last = this.tail();
    if (last && last.t === "agent") last.text += text;
    else this.add({ t: "agent", id: nid(), text });
    this.recentText += text;
  }

  private appendUserChunk(text: string) {
    const last = this.entries[this.entries.length - 1];
    if (last && last.t === "user") last.text += text;
    else this.entries.push({ t: "user", id: nid(), text });
    if (!this.name) this.name = nameFrom(text);
  }

  /** Called with the newest plan or todo list of a chat turn. */
  private chatVersion(kind: "plan" | "todos", value: string | { content: string; status: string }[]) {
    if (this.replaying) return;
    if (kind === "plan" && typeof value === "string") this.addPlanVersion(value, null, this.turnId);
    else if (Array.isArray(value)) this.addTodoVersion(value);
  }

  /** Applies one event. Returns what changed for the app (waiting and notifications). */
  handle(e: SessionEvent): Waiting | "none" {
    if (this.mode === "pane") return this.handlePane(e);
    switch (e.kind) {
      case "starting":
        this.status = "starting";
        this.error = null;
        return "none";
      case "ready":
        this.status = "ready";
        this.acpId = e.acpId;
        this.images = !!e.caps?.images;
        {
          const modes: { id: string }[] = e.modes?.availableModes ?? [];
          const modeOpt = (e.configOptions ?? []).find((o: any) => o.id === "mode" || o.category === "mode");
          const values: string[] = (modeOpt?.options ?? []).map((o: any) => o.value);
          this.planMode = modes.some((m) => m.id === "plan") || values.includes("plan");
        }
        if (Array.isArray(e.configOptions)) this.configOptions = e.configOptions;
        return "none";
      case "failed":
        this.status = "failed";
        this.error = e.message;
        return "none";
      case "signin":
        // Chat mode does not offer a Claude account sign-in (see agent::ClaudeRoute).
        this.status = "claudeAccess";
        this.access = "signedOut";
        this.route = "pane";
        return "none";
      case "claude_route":
        this.status = "claudeAccess";
        this.access = e.access;
        this.route = e.route;
        return "none";
      case "stopped":
        this.status = "stopped";
        this.thinking = false;
        return "none";
      case "update":
        this.onUpdate(e.update);
        return "none";
      case "permission":
        return this.onPermission(e);
      case "permission_done": {
        for (const en of this.entries) {
          if (en.t === "permission" && en.requestId === e.requestId && en.state === "open") en.state = "answered";
        }
        if (this.waiting === "permission" || this.waiting === "plan") this.waiting = null;
        return "none";
      }
      case "run_started": {
        const p = this.entries.find((x) => x.t === "plan" && x.requestId === e.requestId);
        if (p && p.t === "plan") {
          p.state = "running";
          p.turnId = e.turnId;
        }
        this.waiting = null;
        return "none";
      }
      case "turn_started": {
        this.running = true;
        this.stopping = false;
        this.turnId = e.turnId;
        this.todo = [];
        this.recentText = "";
        if (e.continued) {
          const p = this.entries.find((x) => x.t === "plan" && x.turnId === e.turnId);
          if (p && p.t === "plan") {
            p.state = "running";
            p.result = null;
          }
        } else if (e.planFirst) {
          this.entries.push({
            t: "plan",
            id: nid(),
            requestId: null,
            text: "",
            plan: null,
            state: "planning",
            turnId: e.turnId,
            touched: [],
            outside: [],
            result: null,
            undo: null,
          });
        }
        return "none";
      }
      case "turn_ended":
        return this.onTurnEnded(e as unknown as TurnResult & SessionEvent);
      case "error":
        this.note(e.message, "error");
        return "none";
    }
    return "none";
  }

  private onUpdate(u: any) {
    if (!u) return;
    switch (u.sessionUpdate) {
      case "user_message_chunk":
        if (u.content?.type === "text") this.appendUserChunk(u.content.text);
        break;
      case "agent_message_chunk":
        this.thinking = false;
        if (u.content?.type === "text") this.appendAgent(u.content.text);
        break;
      case "agent_thought_chunk":
        this.thinking = true;
        break;
      case "tool_call": {
        this.thinking = false;
        if (u.kind) this.toolKinds.set(u.toolCallId, u.kind);
        if (u.kind === "switch_mode") break;
        this.add({
          t: "tool",
          id: nid(),
          toolCallId: u.toolCallId,
          kind: u.kind ?? "other",
          title: u.title ?? "",
          status: u.status ?? "pending",
          paths: (u.locations ?? []).map((l: any) => l.path),
        });
        this.scope(u);
        break;
      }
      case "tool_call_update": {
        const kind = this.toolKinds.get(u.toolCallId);
        if (kind === "switch_mode") break;
        const en = this.entries.findLast((x) => x.t === "tool" && x.toolCallId === u.toolCallId);
        if (en && en.t === "tool") {
          if (u.title && u.title !== "Terminal") en.title = u.title;
          if (u.status) en.status = u.status;
          for (const l of u.locations ?? []) if (!en.paths.includes(l.path)) en.paths.push(l.path);
        }
        this.scope(u);
        break;
      }
      case "plan":
        this.todo = (u.entries ?? []).map((x: any) => ({ content: x.content, status: x.status }));
        this.chatVersion("todos", this.todo);
        break;
      case "available_commands_update":
        this.commands = u.availableCommands ?? [];
        break;
      case "config_option_update":
        if (Array.isArray(u.configOptions)) this.configOptions = u.configOptions;
        break;
      case "usage_update":
        if (typeof u.used === "number" && typeof u.size === "number") this.usage = { used: u.used, size: u.size };
        break;
      case "session_info_update":
        if (typeof u.title === "string") this.setTitle(u.title.trim());
        break;
    }
  }

  /** Scope guard: writes to files that are not in the running plan show up in amber. */
  private scope(u: any) {
    const kind = this.toolKinds.get(u.toolCallId);
    if (kind !== "edit" && kind !== "delete" && kind !== "move") {
      return;
    }
    const plan = this.entries.findLast((x) => x.t === "plan" && x.state === "running");
    if (!plan || plan.t !== "plan" || !plan.plan) return;
    const planned = new Set(plan.plan.files.map((f) => f.path.toLowerCase()));
    for (const l of u.locations ?? []) {
      const rel = relTo(this.root, l.path);
      if (rel === null || rel === "") continue;
      if (planned.has(rel.toLowerCase())) {
        if (!plan.touched.includes(rel)) plan.touched.push(rel);
      } else if (!plan.outside.some((o) => o.path === rel)) {
        plan.outside.push({ path: rel, reason: reasonFrom(this.recentText) });
      }
    }
    this.recentText = "";
  }

  private onPermission(e: SessionEvent): Waiting {
    if (e.isPlan) {
      const plan: ParsedPlan = e.plan;
      let card = this.entries.findLast((x) => x.t === "plan" && x.state === "planning");
      if (!card || card.t !== "plan") {
        card = {
          t: "plan",
          id: nid(),
          requestId: null,
          text: "",
          plan: null,
          state: "planning",
          turnId: this.turnId,
          touched: [],
          outside: [],
          result: null,
          undo: null,
        };
        this.entries.push(card);
        card = this.entries[this.entries.length - 1] as Extract<Entry, { t: "plan" }>;
      }
      // The waiting plan goes last, after the lines written while planning.
      const at = this.entries.indexOf(card);
      if (at >= 0 && at !== this.entries.length - 1) {
        const [moved] = this.entries.splice(at, 1);
        this.entries.push(moved);
        card = this.entries[this.entries.length - 1] as Extract<Entry, { t: "plan" }>;
      }
      if (card.t === "plan") {
        card.requestId = e.requestId;
        card.text = e.planText ?? "";
        card.plan = plan;
        card.state = "waiting";
        this.chatVersion("plan", card.text);
      }
      this.waiting = "plan";
      return "plan";
    }
    const tc = e.toolCall ?? {};
    this.entries.push({
      t: "permission",
      id: nid(),
      requestId: e.requestId,
      title: tc.title ?? "",
      kind: tc.kind ?? this.toolKinds.get(tc.toolCallId) ?? "other",
      options: e.options as PermissionOption[],
      state: "open",
    });
    this.waiting = "permission";
    return "permission";
  }

  private onTurnEnded(r: TurnResult): Waiting | "none" {
    this.running = false;
    this.stopping = false;
    this.thinking = false;
    this.turnId = null;
    this.todo = [];
    this.updated = Date.now();
    for (const en of this.entries) {
      if (en.t === "permission" && en.state === "open") en.state = "cancelled";
    }
    if (r.stopReason === "authRequired") {
      // Claude asked for a login during the prompt. A claude_route event shows the route
      // ("Open Claude pane"); the turn leaves no card and no error line.
      const idx = this.entries.findLastIndex((x) => x.t === "plan" && x.turnId === r.turnId && (x.state === "planning" || x.state === "waiting"));
      if (idx >= 0) this.entries.splice(idx, 1);
      if (r.hasSnapshot && r.changes?.files.length) this.entries.push({ t: "changes", id: nid(), turnId: r.turnId, result: r, undo: null });
      this.waiting = null;
      return "none";
    }
    const failed = r.stopReason === "error" || !!r.error;
    const plan = this.entries.findLast((x) => x.t === "plan" && x.turnId === r.turnId);
    if (plan && plan.t === "plan") {
      if (plan.state === "running") {
        plan.result = r;
        if (r.stopReason === "cancelled") plan.state = "paused";
        else plan.state = failed ? "failed" : "done";
      } else if (plan.state === "planning" || plan.state === "waiting") {
        if (failed) {
          plan.state = "failed";
          plan.result = r;
        } else if (r.stopReason === "cancelled" || plan.state === "waiting") {
          plan.state = "discarded";
        } else {
          // The agent answered without a plan (a question, or nothing to change).
          const idx = this.entries.indexOf(plan);
          if (idx >= 0) this.entries.splice(idx, 1);
          if (r.hasSnapshot && r.changes?.files.length) {
            this.entries.push({ t: "changes", id: nid(), turnId: r.turnId, result: r, undo: null });
          }
        }
      }
    } else if (r.hasSnapshot && (r.changes?.files.length || r.changes?.generated.length)) {
      this.entries.push({ t: "changes", id: nid(), turnId: r.turnId, result: r, undo: null });
    }
    if (r.stopReason === "cancelled" && !(plan && plan.t === "plan" && plan.state === "paused")) {
      // Plan discards show their own line.
      if (!(plan && plan.t === "plan" && plan.state === "discarded")) this.note("Stopped by you.");
    }
    if (failed && r.error) this.note(r.error, "error");
    if (r.snapshotError) this.note(r.snapshotError, "error");
    const w: Waiting = failed ? "failed" : "done";
    this.waiting = w;
    return w;
  }

  /** The newest plan for the plan panel: a pane plan from hooks, or a chat plan card. */
  get latestPlan(): { text: string; approved: boolean | null } | null {
    const p = this.entries.findLast((x) => x.t === "panePlan" || (x.t === "plan" && !!x.text));
    if (!p) return null;
    if (p.t === "panePlan") return { text: p.text, approved: p.approved };
    return p.t === "plan" ? { text: p.text, approved: null } : null;
  }

  /** The todo list for the plan panel: from hooks in a pane, from the agent's plan updates in chat. */
  get todos(): { content: string; status: string }[] {
    return this.mode === "pane" ? this.paneTodos : this.todo;
  }

  /**
   * Events of a pane session. Turns come from hooks only: a prompt starts one, Stop ends it
   * (see pane.rs). A pane without hooks records no turns.
   */
  private handlePane(e: SessionEvent): Waiting | "none" {
    switch (e.kind) {
      case "pane_started":
        this.status = "ready";
        this.paneAlive = true;
        this.paneWorking = false;
        this.hooksExpected = !!e.hooksExpected;
        // A new TUI starts in its launch mode: plan with Plan first, else unknown until a hook
        // event or the status line reports it.
        this.paneMode = this.panePlan ? "plan" : null;
        break;
      case "pane_exited":
        this.paneAlive = false;
        this.paneWorking = false;
        this.tracking = "off";
        this.turnId = null;
        break;
      case "hooks_live":
        this.hooksLive = true;
        break;
      case "pane_working":
        this.paneWorking = true;
        break;
      case "pane_idle": {
        this.paneWorking = false;
        if (this.tracking !== "error") this.tracking = "waiting";
        this.turnId = null;
        const last = this.prompts[this.prompts.length - 1];
        if (last && typeof e.lastMessage === "string" && last.answer === null) {
          last.answer = e.lastMessage;
          this.changed();
        }
        break;
      }
      case "pane_mode":
        if (typeof e.mode === "string") this.paneMode = e.mode;
        break;
      case "pane_todos":
        this.paneTodos = Array.isArray(e.todos) ? e.todos : [];
        this.addTodoVersion(this.paneTodos);
        break;
      case "pane_prompt": {
        const text = typeof e.text === "string" ? e.text : "";
        this.prompts.push({ id: nid(), at: Date.now(), prompt: text, answer: null, turnId: null, result: null, undo: null });
        if (this.prompts.length > MAX_PROMPTS) this.prompts.splice(0, this.prompts.length - MAX_PROMPTS);
        this.nameFromPrompt(text);
        this.changed();
        break;
      }
      case "pane_plan": {
        const plan: ParsedPlan = e.plan;
        const turnId: string | null = e.turnId ?? null;
        const open = this.entries.findLast((x) => x.t === "panePlan" && x.turnId === turnId && !x.approved);
        if (open && open.t === "panePlan") {
          open.plan = plan;
          open.text = e.planText ?? "";
          open.approved = !!e.approved;
        } else {
          this.entries.push({ t: "panePlan", id: nid(), turnId, text: e.planText ?? "", plan, approved: !!e.approved });
        }
        this.addPlanVersion(e.planText ?? "", !!e.approved, turnId);
        break;
      }
      case "turn_started": {
        this.tracking = "on";
        this.trackingError = null;
        this.untrackedBy = null;
        this.turnId = e.turnId;
        const last = this.prompts[this.prompts.length - 1];
        if (last && !last.turnId) {
          last.turnId = e.turnId;
          this.changed();
        }
        break;
      }
      case "turn_untracked": {
        this.tracking = e.reason === "otherRunning" ? "otherRunning" : "error";
        this.trackingError = e.error ?? null;
        if (e.reason !== "otherRunning") break;
        // The app adds the other session's name (holderName). The status changes again at the
        // next Stop, so the plan panel keeps a note.
        const by: string = typeof e.holderName === "string" ? e.holderName : "";
        this.untrackedBy = by;
        this.note(untrackedText(by), "warn");
        const last = this.prompts[this.prompts.length - 1];
        if (last && !last.turnId) {
          last.untracked = by;
          this.changed();
        }
        break;
      }
      case "turn_ended": {
        const r = e as unknown as TurnResult;
        this.tracking = "off";
        this.turnId = null;
        this.updated = Date.now();
        if (r.hasSnapshot && (r.changes?.files.length || r.changes?.generated.length)) {
          this.entries.push({ t: "changes", id: nid(), turnId: r.turnId, result: r, undo: null });
        }
        const turn = this.prompts.findLast((x) => x.turnId === r.turnId);
        if (turn) {
          turn.result = $state.snapshot(r) as TurnResult;
          this.changed();
        }
        if (r.failure != null) this.note(r.failure ? t("pane.stopFailure", { error: r.failure }) : t("pane.stopFailureUnknown"), "warn");
        if (r.snapshotError) this.note(t("changes.snapshotFailed", { error: r.snapshotError }), "error");
        break;
      }
    }
    return "none";
  }

  /** Out-of-plan rows for a plan card or a changes card, from the real disk diff. */
  static notInPlan(plan: ParsedPlan | null, outside: OutOfPlan[], path: string): string | null {
    if (!plan) return null;
    const planned = new Set(plan.files.map((f) => f.path.toLowerCase()));
    if (planned.has(path.toLowerCase())) return null;
    return outside.find((o) => o.path === path)?.reason ?? "";
  }
}

/** The note for a prompt that was not tracked because another session held the project. */
export function untrackedText(by: string): string {
  return by ? t("pane.notTracked", { name: by }) : t("pane.notTrackedOther");
}

/** Session name from the first prompt: the first line, short. */
export function nameFrom(text: string): string {
  const line = text.trim().split("\n")[0].replace(/\s+/g, " ");
  if (line.length <= 48) return line.charAt(0).toUpperCase() + line.slice(1);
  const cut = line.slice(0, 48);
  const sp = cut.lastIndexOf(" ");
  return (sp > 24 ? cut.slice(0, sp) : cut).replace(/^./, (c) => c.toUpperCase());
}
