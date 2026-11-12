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

export interface FileVersions {
  path: string;
  before: string | null;
  after: string | null;
  binary: boolean;
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

export type Entry =
  | { t: "user"; id: string; text: string; images?: number; files?: string[] }
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
    }
  | { t: "changes"; id: string; turnId: string; result: TurnResult; undo: UndoState | null }
  | { t: "note"; id: string; text: string; tone: "muted" | "error" | "warn" };

export interface UndoState {
  outcome: UndoOutcome;
  redone?: boolean;
}

export type Waiting = null | "plan" | "permission" | "done" | "failed";
