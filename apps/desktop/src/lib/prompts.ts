// Pure helpers for the Prompts panel and the dot strip of the agent pane: the state of a
// prompt, search, the dots when a session has many prompts, stepping from prompt to prompt,
// and the turns that "Undo back to this prompt" reverts. No DOM and no stores, so
// scripts/prompts.test.mjs runs them with `node --test`.
import type { PromptTurn } from "./types";

/** What a dot shows. In order of weight: a bucket of dots shows the heaviest state in it. */
export type PromptState = "working" | "failed" | "changed" | "undone" | "untracked" | "done";

const WEIGHT: Record<PromptState, number> = { working: 5, failed: 4, changed: 3, undone: 2, untracked: 1, done: 0 };

export function fileCount(p: PromptTurn): number {
  return p.result?.changes?.files.length ?? 0;
}

/** `last` and `busy`: the newest prompt of a session that works now has no result yet. */
export function promptState(p: PromptTurn, last: boolean, busy: boolean): PromptState {
  if (last && busy && !p.result) return "working";
  const r = p.result;
  if (r && (r.error || (r.failure !== undefined && r.failure !== null))) return "failed";
  if (p.undo) return "undone";
  if (fileCount(p)) return "changed";
  if (p.untracked != null) return "untracked";
  return "done";
}

/** The first line of a text, cut to `max` characters. Empty text gives "". */
export function firstLine(text: string, max = 80): string {
  const line = text.trim().split("\n")[0]?.trim() ?? "";
  return line.length > max ? line.slice(0, max - 3) + "..." : line;
}

/** Start and end (UTF-16) of every match of `query` in `text`, without case. Matches do not overlap. */
export function matchRanges(text: string, query: string): [number, number][] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const hay = text.toLowerCase();
  const out: [number, number][] = [];
  // Lowercase can change the length of a few characters (for example U+0130). Then the
  // positions would not match the text, so only plain matches are used.
  if (hay.length !== text.length) return out;
  for (let i = hay.indexOf(q); i >= 0; i = hay.indexOf(q, i + q.length)) out.push([i, i + q.length]);
  return out;
}

/** Splits a text into parts with and without a match, for highlighting. */
export function splitMatches(text: string, query: string): { text: string; hit: boolean }[] {
  const ranges = matchRanges(text, query);
  if (!ranges.length) return [{ text, hit: false }];
  const out: { text: string; hit: boolean }[] = [];
  let at = 0;
  for (const [a, b] of ranges) {
    if (a > at) out.push({ text: text.slice(at, a), hit: false });
    out.push({ text: text.slice(a, b), hit: true });
    at = b;
  }
  if (at < text.length) out.push({ text: text.slice(at), hit: false });
  return out;
}

/** Indexes of the prompts whose prompt or answer has the query. An empty query keeps all. */
export function filterPrompts(prompts: PromptTurn[], query: string): number[] {
  const q = query.trim().toLowerCase();
  const out: number[] = [];
  prompts.forEach((p, i) => {
    if (!q || p.prompt.toLowerCase().includes(q) || (p.answer ?? "").toLowerCase().includes(q)) out.push(i);
  });
  return out;
}

/**
 * The next prompt to show from `visible` (indexes, in order). `current` is an index into all
 * prompts, or -1 for none. "prev" and "next" stop at the ends; with no current prompt (or one
 * that the search hides), "next" goes to the first visible prompt after it and "prev" to the
 * last one before it. Returns -1 when nothing is visible.
 */
export function stepPrompt(visible: number[], current: number, move: "prev" | "next" | "first" | "last"): number {
  if (!visible.length) return -1;
  if (move === "first") return visible[0];
  if (move === "last") return visible[visible.length - 1];
  const at = visible.indexOf(current);
  if (at >= 0) return visible[Math.max(0, Math.min(visible.length - 1, at + (move === "next" ? 1 : -1)))];
  if (current < 0) return move === "next" ? visible[0] : visible[visible.length - 1];
  if (move === "next") return visible.find((i) => i > current) ?? visible[visible.length - 1];
  return visible.findLast((i) => i < current) ?? visible[0];
}

/** One dot of the strip: prompts `start` to `end` (both included), and the heaviest state. */
export interface Dot {
  start: number;
  end: number;
  state: PromptState;
}

/**
 * The dots for `states` (one per prompt) when the strip has room for `room` dots. Up to
 * `room` prompts, one dot each. More: each dot holds the same number of prompts (the last one
 * may hold fewer), so 200 prompts in 50 dots are 4 per dot.
 */
export function bucketDots(states: PromptState[], room: number): Dot[] {
  const n = states.length;
  if (!n) return [];
  const size = Math.max(1, Math.ceil(n / Math.max(1, Math.floor(room))));
  const out: Dot[] = [];
  for (let start = 0; start < n; start += size) {
    const end = Math.min(n, start + size) - 1;
    let state: PromptState = states[start];
    for (let i = start + 1; i <= end; i++) if (WEIGHT[states[i]] > WEIGHT[state]) state = states[i];
    out.push({ start, end, state });
  }
  return out;
}

/** The dot that holds prompt `index`, or -1. */
export function dotOf(dots: Dot[], index: number): number {
  return dots.findIndex((d) => index >= d.start && index <= d.end);
}

/**
 * "Undo back to this prompt": the later prompts whose turn can be undone (tracked, with
 * changes, not undone yet), newest first, which is the order to undo them in.
 */
export function undoBackTurns(prompts: PromptTurn[], index: number): PromptTurn[] {
  const out: PromptTurn[] = [];
  for (let i = prompts.length - 1; i > index; i--) {
    const p = prompts[i];
    if (p.turnId && !p.undo && p.untracked == null && fileCount(p)) out.push(p);
  }
  return out;
}

/** The files those turns changed, each once, in path order. */
export function undoBackFiles(turns: PromptTurn[]): string[] {
  const set = new Set<string>();
  for (const p of turns) for (const f of p.result?.changes?.files ?? []) set.add(f.path);
  return [...set].sort();
}
