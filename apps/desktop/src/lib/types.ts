// Shapes shared with the Rust side (serde camelCase).

export interface ProjectInfo {
  id: string;
  root: string;
  name: string;
  monogram: string;
  hue: number;
  isGit: boolean;
  branch: string | null;
}

export type PlanChange = "create" | "edit" | "delete";

export interface PlanFile {
  change: PlanChange;
  path: string;
  reason: string;
  found: boolean;
}

export interface ParsedPlan {
  steps: string[];
  files: PlanFile[];
  risk: string | null;
  fallback: boolean;
}

export type ChangeStatus = "create" | "edit" | "delete" | "rename";

export interface FileChange {
  path: string;
  oldPath?: string;
  status: ChangeStatus;
  added: number;
  removed: number;
  binary: boolean;
}

export interface Changes {
  files: FileChange[];
  generated: { folder: string; count: number }[];
  added: number;
  removed: number;
}

export type Uncovered = "nestedRepo" | "bigFile" | "symlink" | "unreadable";

export interface NotCovered {
  path: string;
  why: Uncovered;
}

export interface PermissionOption {
  optionId: string;
  name: string;
  kind: string;
}

export interface UndoPreview {
  files: string[];
  conflicts: string[];
  yourEdits: string[];
  commands: string[];
  notCovered: NotCovered[];
  ignoredChanged: string[];
}

export interface UndoOutcome {
  restored: string[];
  kept: string[];
  failed: [string, string][];
  exact: boolean;
}

export type HunkState = "agent" | "undone" | "changed";

/** One run of changed lines (crates/core/src/hunks.rs). Lines are 0-based. */
export interface Hunk {
  oldStart: number;
  oldLen: number;
  newStart: number;
  newLen: number;
  state: HunkState;
}

/** One file of a turn for the review (hunks::FileDiff). */
export interface FileDiff {
  path: string;
  oldPath?: string;
  binary: boolean;
  before: string | null;
  after: string | null;
  hunks: Hunk[];
  /** The file on disk against the turn's end. */
  current: "agent" | "changed" | "missing";
  /** Undo change works per hunk (both versions are text). */
  hunkUndo: boolean;
}

export interface FileVersions {
  path: string;
  before: string | null;
  after: string | null;
  binary: boolean;
}

/** Turn summary without a model call (crates/core/src/summary.rs). */
export interface TurnSummary {
  files: number;
  created: number;
  deleted: number;
  added: number;
  removed: number;
  generated: number;
  commandCount: number;
  commands: string[];
  planSteps: string[];
  planStepCount: number;
  /** Start of the agent's last message (panes with hooks). */
  excerpt: string | null;
}

/** Result of a finished turn, from the `turn_ended` event. */
export interface TurnResult {
  turnId: string;
  stopReason: string;
  error: string | null;
  hasSnapshot: boolean;
  changes?: Changes;
  notCovered?: NotCovered[];
  commands?: string[];
  outside?: boolean;
  yourEdits?: string[];
  snapshotError?: string;
  summary?: TurnSummary;
  /** Panes: why the turn ended (stop, stopFailure, prompt, exit, close). */
  reason?: string;
  /** Panes: StopFailure error type ("" when unknown). Null for a normal end. */
  failure?: string | null;
}

export interface ConfigOption {
  id: string;
  name: string;
  category?: string;
  currentValue: string;
  options: { value: string; name: string; description?: string }[];
}

export interface AgentCommand {
  name: string;
  description?: string;
  input?: { hint?: string } | null;
}

export type PlanState = "planning" | "waiting" | "running" | "paused" | "done" | "failed" | "discarded";

export interface OutOfPlan {
  path: string;
  reason: string;
}

/** One version of a session's plan, for the plan panel history. `source` "plan": ExitPlanMode
 * (pane) or a plan card (chat). "todos": a todo list whose items changed (TodoWrite, the Task
 * tools or the agent's plan updates); a status change updates the version in place. */
export interface PlanVersion {
  n: number;
  /** Time in ms. */
  at: number;
  source: "plan" | "todos";
  text: string;
  /** Pane plans: approved in the TUI. Null for chat plans and todo lists. */
  approved: boolean | null;
  turnId: string | null;
  todos?: { content: string; status: string }[];
}

/** One prompt and its answer, for the Prompts panel. Pane prompts come from hooks
 * (UserPromptSubmit and Stop) and are kept in app data; chat prompts come from the
 * conversation. */
export interface PromptTurn {
  id: string;
  /** Time in ms. Null when unknown (a chat history replayed after a resume). */
  at: number | null;
  prompt: string;
  answer: string | null;
  turnId: string | null;
  result: TurnResult | null;
  undo: UndoState | null;
  /** Not tracked: the name of the session that held the project ("" when unknown). */
  untracked?: string | null;
}

export type Entry =
  | { t: "user"; id: string; text: string; images?: number; files?: string[]; at?: number }
  | { t: "agent"; id: string; text: string }
  | { t: "tool"; id: string; toolCallId: string; kind: string; title: string; status: string; paths: string[] }
  | {
      t: "permission";
      id: string;
      requestId: string;
      title: string;
      kind: string;
      options: PermissionOption[];
      state: "open" | "answered" | "cancelled";
      answer?: string;
    }
  | {
      t: "plan";
      id: string;
      requestId: string | null;
      text: string;
      plan: ParsedPlan | null;
      state: PlanState;
      turnId: string | null;
      touched: string[];
      outside: OutOfPlan[];
      result: TurnResult | null;
      undo: UndoState | null;
      /** Files the user marked as reviewed (Keep). Missing: a turn from before reviews. */
      reviewed?: string[];
    }
  | { t: "changes"; id: string; turnId: string; result: TurnResult; undo: UndoState | null; reviewed?: string[] }
  /** A pane plan from the ExitPlanMode hook. Approval happens in the agent's TUI. */
  | { t: "panePlan"; id: string; turnId: string | null; text: string; plan: ParsedPlan; approved: boolean }
  | { t: "note"; id: string; text: string; tone: "muted" | "error" | "warn" };

export interface UndoState {
  outcome: UndoOutcome;
  redone?: boolean;
}

export type Waiting = null | "plan" | "permission" | "done" | "failed";
