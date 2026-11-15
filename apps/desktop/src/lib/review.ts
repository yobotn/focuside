// Review of a turn's changes and the state of a session, as pure functions, so
// scripts/review.test.mjs can run them with Node (type imports only).
// - Diff blocks: context lines, collapsed unchanged lines and hunks, from both versions of a
//   file and the hunks Rust found (crates/core/src/hunks.rs). Inline and side by side use the
//   same blocks.
// - Reviewed files (Keep), the files a session changed, and what to review.
// - The state a session tab and the status bar show.
import type { Entry, FileChange, Hunk, HunkState, TurnResult, Waiting } from "./types";

/** Lines without their line break, as Rust counts them (`split_inclusive('\n')`). */
export function splitLines(text: string | null): string[] {
  if (!text) return [];
  const out = text.split("\n");
  if (out[out.length - 1] === "") out.pop();
  return out.map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l));
}

/** One line in a diff. Line numbers are 1-based; null on the side that does not have it. */
export interface DiffLine {
  old: number | null;
  new: number | null;
  text: string;
}

export type Block =
  | { kind: "context"; lines: DiffLine[] }
  /** Unchanged lines that are not shown. `at` (old line, 0-based) names the gap for expanding. */
  | { kind: "gap"; at: number; count: number }
  | { kind: "hunk"; index: number; state: HunkState; del: DiffLine[]; add: DiffLine[] };

export const CONTEXT = 3;

/**
 * The blocks of a file diff: each hunk with up to CONTEXT unchanged lines around it, and gaps
 * for the unchanged lines in between. A gap whose `at` is in `expanded` shows its lines.
 */
export function diffBlocks(before: string | null, after: string | null, hunks: Hunk[], expanded: ReadonlySet<number> = new Set(), context = CONTEXT): Block[] {
  const a = splitLines(before);
  const b = splitLines(after);
  const out: Block[] = [];
  let i = 0;
  let j = 0;
  const same = (from: number, count: number, offset: number): DiffLine[] =>
    Array.from({ length: count }, (_, k) => ({ old: from + k + 1, new: from + k + offset + 1, text: a[from + k] ?? b[from + k + offset] ?? "" }));
  const pushContext = (lines: DiffLine[]) => {
    if (!lines.length) return;
    const last = out[out.length - 1];
    if (last?.kind === "context") last.lines.push(...lines);
    else out.push({ kind: "context", lines });
  };
  /** Unchanged lines i..end (old side); `first` and `last` say whether a hunk is above or below. */
  const unchanged = (end: number, above: boolean, below: boolean) => {
    const count = end - i;
    if (count <= 0) return;
    const offset = j - i;
    const head = above ? Math.min(context, count) : 0;
    const tail = below ? Math.min(context, count - head) : 0;
    const hidden = count - head - tail;
    if (hidden <= 1 || expanded.has(i + head)) {
      pushContext(same(i, count, offset));
    } else {
      pushContext(same(i, head, offset));
      out.push({ kind: "gap", at: i + head, count: hidden });
      pushContext(same(i + head + hidden, tail, offset));
    }
    j += count;
    i = end;
  };
  hunks.forEach((h, index) => {
    unchanged(h.oldStart, index > 0, true);
    out.push({
      kind: "hunk",
      index,
      state: h.state,
      del: Array.from({ length: h.oldLen }, (_, k) => ({ old: h.oldStart + k + 1, new: null, text: a[h.oldStart + k] ?? "" })),
      add: Array.from({ length: h.newLen }, (_, k) => ({ old: null, new: h.newStart + k + 1, text: b[h.newStart + k] ?? "" })),
    });
    i = h.oldStart + h.oldLen;
    j = h.newStart + h.newLen;
  });
  unchanged(a.length, hunks.length > 0, false);
  return out;
}

/** Side by side rows of a hunk: removed lines left, added lines right, paired by place. */
export function pairRows(del: DiffLine[], add: DiffLine[]): { left: DiffLine | null; right: DiffLine | null }[] {
  return Array.from({ length: Math.max(del.length, add.length) }, (_, k) => ({ left: del[k] ?? null, right: add[k] ?? null }));
}

// Reviewed files and what to review

type TurnEntry = Extract<Entry, { t: "changes" | "plan" }>;

function turnEntries(entries: Entry[]): (TurnEntry & { turnId: string; result: TurnResult })[] {
  const out: (TurnEntry & { turnId: string; result: TurnResult })[] = [];
  for (const e of entries) {
    if ((e.t === "changes" || e.t === "plan") && e.turnId && e.result?.changes?.files.length) out.push(e as TurnEntry & { turnId: string; result: TurnResult });
  }
  return out;
}

/** True when the user kept (reviewed) a file of a turn. Turns from before reviews count as reviewed. */
export function isReviewed(e: { reviewed?: string[] }, path: string): boolean {
  return !e.reviewed || e.reviewed.includes(path);
}

/** Files of turns that are not undone and not reviewed yet. */
export function unreviewedCount(entries: Entry[]): number {
  let n = 0;
  for (const e of turnEntries(entries)) {
    if (e.undo) continue;
    for (const f of e.result.changes!.files) if (!isReviewed(e, f.path)) n++;
  }
  return n;
}

/** The files a session changed in turns that are not undone, each once. */
export function sessionFiles(entries: Entry[]): string[] {
  const seen = new Set<string>();
  for (const e of turnEntries(entries)) {
    if (e.undo) continue;
    for (const f of e.result.changes!.files) seen.add(f.path);
  }
  return [...seen];
}

/** One file to review: a file of one turn. */
export interface ReviewItem {
  turnId: string;
  file: FileChange;
}

/** The files to review: one turn (also when undone), or with null every turn of the session
 * that is not undone, oldest first. */
export function reviewItems(entries: Entry[], turnId: string | null): ReviewItem[] {
  const out: ReviewItem[] = [];
  for (const e of turnEntries(entries)) {
    if (turnId ? e.turnId !== turnId : !!e.undo) continue;
    for (const file of e.result.changes!.files) out.push({ turnId: e.turnId, file });
  }
  return out;
}

/** The new reviewed list of a turn after Keep (`keep`) or after taking it back. */
export function setReviewed(list: string[] | undefined, path: string, keep: boolean): string[] {
  const cur = list ?? [];
  if (keep) return cur.includes(path) ? cur : [...cur, path];
  return cur.filter((p) => p !== path);
}

/** Moves to the next or previous hunk: `at` is the current place in the list of all hunks of
 * the review (-1 for none). Stops at both ends. */
export function stepHunk(at: number, count: number, step: 1 | -1): number {
  if (count <= 0) return -1;
  if (at < 0) return step > 0 ? 0 : count - 1;
  return Math.max(0, Math.min(count - 1, at + step));
}

// Session state

export type TabState = "dormant" | "starting" | "working" | "waiting" | "failed" | "review" | "done" | "idle";

export interface StateInput {
  dormant: boolean;
  busy: boolean;
  waiting: Waiting;
  /** Session status: "failed" when the agent could not start, "starting" while it starts. */
  status: string;
  unreviewed: number;
}

/**
 * What a session tab and the status bar show. Working beats everything, then waiting for the
 * user (amber), a failure (red), changes to review (check), a finished turn, else nothing.
 */
export function tabState(s: StateInput): TabState {
  if (s.dormant) return "dormant";
  if (s.busy) return "working";
  if (s.waiting === "plan" || s.waiting === "permission") return "waiting";
  if (s.waiting === "failed" || s.status === "failed") return "failed";
  if (s.status === "starting") return "starting";
  if (s.unreviewed > 0) return "review";
  if (s.waiting === "done") return "done";
  return "idle";
}

/** Hours, minutes and seconds of a duration in ms. */
export function splitDuration(ms: number): { h: number; m: number; s: number } {
  const total = Math.max(0, Math.floor(ms / 1000));
  return { h: Math.floor(total / 3600), m: Math.floor((total % 3600) / 60), s: total % 60 };
}

// Notifications

export type NotifyMode = "never" | "background" | "always";

/**
 * Whether a session that finished or waits gets a system notification. "background": only while
 * the FocusIDE window is not in front. "always": also in front, except for the session the user
 * looks at.
 */
export function shouldNotify(mode: NotifyMode, focused: boolean, looking: boolean): boolean {
  if (mode === "never") return false;
  if (!focused) return true;
  return mode === "always" && !looking;
}

/** The notify setting from a saved state: the old checkbox `notifications: false` is "never". */
export function loadNotify(raw: unknown, oldFlag: unknown): NotifyMode {
  if (raw === "never" || raw === "background" || raw === "always") return raw;
  return oldFlag === false ? "never" : "background";
}

// Recent commands

/** The palette order: the recent commands first (newest first), then the others in their order. */
export function withRecent<T extends { id: string }>(items: T[], recent: string[]): { recent: T[]; rest: T[] } {
  const byId = new Map(items.map((x) => [x.id, x]));
  const first = recent.map((id) => byId.get(id)).filter((x): x is T => !!x);
  const ids = new Set(first.map((x) => x.id));
  return { recent: first, rest: items.filter((x) => !ids.has(x.id)) };
}

/** The recent list after running `id`: newest first, each once, at most `max`. */
export function pushRecent(recent: string[], id: string, max = 5): string[] {
  return [id, ...recent.filter((x) => x !== id)].slice(0, max);
}
