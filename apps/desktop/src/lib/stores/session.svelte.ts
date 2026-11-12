// One agent session in the UI: its conversation entries, plan cards, waiting state and the
// live scope guard. Fed by `session` events from Rust.
import type { SessionEvent } from "../api";
import type {
  AgentCommand,
  ConfigOption,
  Entry,
  OutOfPlan,
  ParsedPlan,
  PermissionOption,
  TurnResult,
  Waiting,
} from "../types";

export type Status = "idle" | "starting" | "ready" | "stopped" | "failed" | "signin";

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
}

export class Session {
  key: string;
  projectId: string;
  root: string;
  acpId = $state<string | null>(null);
  agent = "claude";
  /** The agent has a plan mode, so "Plan first" can work. */
  planMode = $state(true);
  name = $state("");
  renamed = $state(false);
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
    }
  }

  get model(): ConfigOption | undefined {
    return this.configOptions.find((o) => o.id === "model" || o.category === "model");
  }

  /** True while this session holds its project for a turn that can change files. */
  get claims(): boolean {
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
    return {
      acpId: this.acpId,
      name: this.name,
      renamed: this.renamed,
      created: this.created,
      updated: this.updated,
      agent: this.agent,
    };
  }

  addUser(text: string, images = 0, files: string[] = []) {
    this.entries.push({ t: "user", id: nid(), text, images, files });
    if (!this.name) this.name = nameFrom(text);
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

  /** Applies one event. Returns what changed for the app (waiting and notifications). */
  handle(e: SessionEvent): Waiting | "none" {
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
        this.status = "signin";
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
        if (u.title && !this.renamed) this.name = u.title;
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

  /** Out-of-plan rows for a plan card or a changes card, from the real disk diff. */
  static notInPlan(plan: ParsedPlan | null, outside: OutOfPlan[], path: string): string | null {
    if (!plan) return null;
    const planned = new Set(plan.files.map((f) => f.path.toLowerCase()));
    if (planned.has(path.toLowerCase())) return null;
    return outside.find((o) => o.path === path)?.reason ?? "";
  }
}

/** Session name from the first prompt: the first line, short. */
export function nameFrom(text: string): string {
  const line = text.trim().split("\n")[0].replace(/\s+/g, " ");
  if (line.length <= 48) return line.charAt(0).toUpperCase() + line.slice(1);
  const cut = line.slice(0, 48);
  const sp = cut.lastIndexOf(" ");
  return (sp > 24 ? cut.slice(0, sp) : cut).replace(/^./, (c) => c.toUpperCase());
}
