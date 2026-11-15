// What comes back when FocusIDE opens again: the open session tabs of each project, which
// panels were shown, and the changes cards of each session. Pure functions, so
// scripts/restore.test.mjs can run them with Node (type imports only).
import type { Entry, TurnResult, UndoState } from "./types";

/** The open session tabs of one project: saved session ids (`acpId`) in tab order, and the
 * active one. */
export interface SavedTabs {
  ids: string[];
  active: string | null;
}

/** Which panels were shown. The side bar is `filesVisible`; the viewer shows with a file. */
export interface SavedPanels {
  plan: boolean;
  prompts: boolean;
  terminal: boolean;
}

/** A turn with changes, kept so the changes and Undo come back after a restart. */
export interface SavedTurn {
  turnId: string;
  result: TurnResult;
  undo: UndoState | null;
  /** Chat: the index of the user message the turn belongs to, -1 for none. */
  prompt: number;
  /** Chat: the start of that user message, to find it again in the replayed history. */
  text?: string;
  /** Files the user kept (reviewed). Missing in turns saved before reviews: they count as reviewed. */
  reviewed?: string[];
}

export const MAX_TURNS = 100;

/** Saved tabs from the state file. Unknown shapes are dropped. */
export function loadTabs(raw: unknown): Record<string, SavedTabs> {
  const out: Record<string, SavedTabs> = {};
  if (!raw || typeof raw !== "object") return out;
  for (const [root, v] of Object.entries(raw as Record<string, unknown>)) {
    const x = v as Partial<SavedTabs> | null;
    if (!x || !Array.isArray(x.ids)) continue;
    const ids = [...new Set(x.ids.filter((id): id is string => typeof id === "string" && !!id))];
    out[root] = { ids, active: typeof x.active === "string" && ids.includes(x.active) ? x.active : null };
  }
  return out;
}

/** Saved panel visibility. Anything missing is hidden. */
export function loadPanels(raw: unknown): SavedPanels {
  const x = (raw && typeof raw === "object" ? raw : {}) as Partial<Record<keyof SavedPanels, unknown>>;
  return { plan: x.plan === true, prompts: x.prompts === true, terminal: x.terminal === true };
}

/** The tabs to save: open sessions worth listing (`saved()` is not null), in tab order. */
export function tabsOf(sessions: { key: string; saved(): { acpId: string } | null }[], activeKey: string | null): SavedTabs {
  const ids: string[] = [];
  let active: string | null = null;
  for (const s of sessions) {
    const id = s.saved()?.acpId;
    if (!id || ids.includes(id)) continue;
    ids.push(id);
    if (s.key === activeKey) active = id;
  }
  return { ids, active };
}

/** A turn with changes to show in a changes card. */
export function hasChanges(r: TurnResult | null | undefined): r is TurnResult {
  return !!r?.hasSnapshot && !!(r.changes?.files.length || r.changes?.generated.length);
}

/** The changes cards of a conversation with the user message each one belongs to, oldest
 * first. `copy` makes a plain copy of reactive data ($state.snapshot in the app). */
export function turnsOf(entries: Entry[], copy: <T>(x: T) => T = (x) => x): SavedTurn[] {
  const out: SavedTurn[] = [];
  let prompt = -1;
  let text = "";
  for (const e of entries) {
    if (e.t === "user") {
      prompt++;
      text = e.text.slice(0, 200);
    } else if ((e.t === "changes" || e.t === "plan") && e.turnId && hasChanges(e.result)) {
      const turn: SavedTurn = { turnId: e.turnId, result: copy(e.result), undo: e.undo ? copy(e.undo) : null, prompt };
      if (prompt >= 0) turn.text = text;
      if (e.reviewed) turn.reviewed = [...e.reviewed];
      out.push(turn);
    }
  }
  return out;
}

/** Saved turns from app data. Unknown shapes are dropped. */
export function loadTurns(raw: unknown): SavedTurn[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((x): x is SavedTurn => !!x && typeof x === "object" && typeof x.turnId === "string" && hasChanges(x.result))
    .map((x) => {
      const t: SavedTurn = { ...x, prompt: typeof x.prompt === "number" ? x.prompt : -1, undo: x.undo ?? null };
      if (Array.isArray(x.reviewed)) t.reviewed = x.reviewed.filter((p) => typeof p === "string");
      else delete t.reviewed;
      return t;
    })
    .slice(-MAX_TURNS);
}

/**
 * Puts saved changes cards into a conversation, each after the user message it belongs to:
 * at its place when the text there matches, else at the first message with that text. A card
 * whose message is not found (or that has none) goes at the end.
 */
export function placeTurns(entries: Entry[], turns: SavedTurn[], newId: () => string) {
  for (const x of turns) {
    const users = entries.map((e, i) => (e.t === "user" ? i : -1)).filter((i) => i >= 0);
    const same = (i: number | undefined) => {
      const e = i === undefined ? undefined : entries[i];
      return !!e && e.t === "user" && (!x.text || e.text.slice(0, 200) === x.text);
    };
    let at = x.prompt >= 0 && same(users[x.prompt]) ? x.prompt : -1;
    if (at < 0 && x.prompt >= 0 && x.text) at = users.findIndex((i) => same(i));
    const card: Entry = { t: "changes", id: newId(), turnId: x.turnId, result: x.result, undo: x.undo ?? null };
    if (x.reviewed) card.reviewed = x.reviewed;
    const next = at >= 0 ? users[at + 1] : undefined;
    if (at >= 0 && next !== undefined) entries.splice(next, 0, card);
    else entries.push(card);
  }
}
