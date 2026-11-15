// The panel layout. Four dock zones hold groups, and a group holds one or more panels as tabs.
// Left and right run the full height. Center groups sit side by side in the middle; one of them
// fills the space that is left (the group with the Session panel, else the first one). Bottom
// runs under the center groups. Every panel can move, the Session panel too (the agent pane or
// the conversation of the active session).
// Pure functions only, so the layout can be checked and changed without the UI.

export type PanelId = "session" | "side" | "search" | "git" | "viewer" | "plan" | "prompts" | "terminal";
export type Zone = "left" | "center" | "right" | "bottom";

export const PANELS: PanelId[] = ["session", "side", "search", "git", "viewer", "plan", "prompts", "terminal"];
export const ZONES: Zone[] = ["left", "center", "right", "bottom"];

/** Panels in one place. `size` is the width in pixels (not used in the bottom zone). */
export interface DockGroup {
  id: string;
  panels: PanelId[];
  active: PanelId;
  size: number;
}

/** Groups per zone, in screen order (left to right). */
export type Dock = Record<Zone, DockGroup[]>;

/**
 * Where a dragged panel lands: a new group in a zone, or a tab in a group. A new group goes to
 * the outer edge (left, right), last (bottom), or in the center right after the group `after`
 * (default: the group with the Session panel, else last).
 */
export type DropTarget = { kind: "zone"; zone: Zone; after?: string } | { kind: "group"; zone: Zone; id: string };

/** The Session panel keeps at least 360 px wherever it is (PLAN.md section 5). */
export const MIN_WIDTH: Record<PanelId, number> = { session: 360, side: 160, search: 220, git: 220, viewer: 320, plan: 280, prompts: 240, terminal: 240 };
export const DEFAULT_WIDTH: Record<PanelId, number> = { session: 640, side: 248, search: 320, git: 300, viewer: 640, plan: 640, prompts: 320, terminal: 480 };
export const DEFAULT_BOTTOM = 260;

let seq = 0;
export function groupId(): string {
  seq += 1;
  return `g${Date.now().toString(36)}${seq}`;
}

function group(p: PanelId, size = DEFAULT_WIDTH[p]): DockGroup {
  return { id: groupId(), panels: [p], active: p, size };
}

/**
 * Side bar on the left, with Search and Source control (git) as more tabs. In the center the Session panel, then the
 * viewer, the plan and the prompts as tabs (the editor area). Terminal below. Search, plan,
 * prompts, source control and terminal stay hidden until they open.
 */
export function defaultDock(): Dock {
  const editor: DockGroup = { id: groupId(), panels: ["viewer", "plan", "prompts"], active: "viewer", size: DEFAULT_WIDTH.viewer };
  const side: DockGroup = { id: groupId(), panels: ["side", "search", "git"], active: "side", size: DEFAULT_WIDTH.side };
  return { left: [side], center: [group("session"), editor], right: [], bottom: [group("terminal")] };
}

/** The dock from the layout saved before docking (files and viewer on a side, terminal below). */
export function legacyDock(old: { filesSide?: unknown; viewerSide?: unknown; filesWidth?: unknown; viewerWidth?: unknown }): Dock {
  const d: Dock = { left: [], center: [], right: [], bottom: [group("terminal")] };
  const width = (v: unknown, p: PanelId) => (typeof v === "number" && v >= MIN_WIDTH[p] ? v : DEFAULT_WIDTH[p]);
  const files = group("side", width(old.filesWidth, "side"));
  const viewer: DockGroup = { ...group("viewer", width(old.viewerWidth, "viewer")), panels: ["viewer", "plan"] };
  // Two panels on one side: the file tree sat on the outside. A viewer on the right sat next
  // to the conversation, which is the editor area now.
  if (old.filesSide === "right") d.right.push(files);
  else d.left.push(files);
  if (old.viewerSide === "left") d.left.push(viewer);
  else d.center.push(viewer);
  // The Session panel and Prompts did not exist then. They go to their default places.
  return normalizeDock(d)!;
}

/** A saved dock as it can be trusted: known panels, each exactly once, no empty groups. */
export function normalizeDock(raw: unknown): Dock | null {
  if (!raw || typeof raw !== "object") return null;
  const seen = new Set<PanelId>();
  const out: Dock = { left: [], center: [], right: [], bottom: [] };
  for (const zone of ZONES) {
    const list = (raw as Record<string, unknown>)[zone];
    if (!Array.isArray(list)) continue;
    for (const g of list) {
      if (!g || typeof g !== "object") continue;
      const panels = (Array.isArray(g.panels) ? g.panels : []).filter(
        (p: unknown): p is PanelId => PANELS.includes(p as PanelId) && !seen.has(p as PanelId),
      );
      if (!panels.length) continue;
      panels.forEach((p: PanelId) => seen.add(p));
      const size = typeof g.size === "number" && Number.isFinite(g.size) ? g.size : DEFAULT_WIDTH[panels[0] as PanelId];
      out[zone].push({
        id: typeof g.id === "string" && g.id ? g.id : groupId(),
        panels,
        active: panels.includes(g.active) ? g.active : panels[0],
        size: Math.max(size, ...panels.map((p: PanelId) => MIN_WIDTH[p])),
      });
    }
  }
  // A panel missing from the saved dock (for example one added in a later version) goes back to
  // its default place: a tab next to a panel of its default group, or a new group. Docks saved
  // before the Session panel could move had the session area before all center groups, so the
  // Session panel goes first in the center and such a layout looks the same as before.
  const home = defaultDock();
  for (const p of PANELS) {
    if (seen.has(p)) continue;
    const at = findPanel(home, p)!;
    const mate = at.group.panels.map((x) => findPanel(out, x)).find((x) => !!x);
    if (mate) mate.group.panels.push(p);
    else if (at.zone === "left" || at.zone === "center") out[at.zone].unshift(group(p));
    else out[at.zone].push(group(p));
  }
  return out;
}

export function findPanel(d: Dock, p: PanelId): { zone: Zone; group: DockGroup } | null {
  for (const zone of ZONES) {
    const g = d[zone].find((x) => x.panels.includes(p));
    if (g) return { zone, group: g };
  }
  return null;
}

export function findGroup(d: Dock, id: string): { zone: Zone; group: DockGroup } | null {
  for (const zone of ZONES) {
    const g = d[zone].find((x) => x.id === id);
    if (g) return { zone, group: g };
  }
  return null;
}

/** The center group that fills the free space: the one with the Session panel, else the first. */
export function fillGroup(list: DockGroup[]): DockGroup | null {
  return list.find((g) => g.panels.includes("session")) ?? list[0] ?? null;
}

function clone(d: Dock): Dock {
  return {
    left: d.left.map((g) => ({ ...g, panels: [...g.panels] })),
    center: d.center.map((g) => ({ ...g, panels: [...g.panels] })),
    right: d.right.map((g) => ({ ...g, panels: [...g.panels] })),
    bottom: d.bottom.map((g) => ({ ...g, panels: [...g.panels] })),
  };
}

/** The panels of each group in screen order, without ids, sizes or active tabs. */
function shape(d: Dock): string {
  return JSON.stringify(ZONES.map((z) => d[z].map((g) => g.panels)));
}

/** True when the drop would leave the layout as it is. */
export function isNoop(d: Dock, p: PanelId, to: DropTarget): boolean {
  const from = findPanel(d, p);
  if (!from) return false;
  if (to.kind === "group") return to.id === from.group.id;
  if (from.group.panels.length > 1) return false;
  return shape(place(d, p, to)) === shape(d);
}

/** Moves a panel to a drop target and returns the new dock. The panel becomes the active tab. */
export function movePanel(d: Dock, p: PanelId, to: DropTarget): Dock {
  if (isNoop(d, p, to)) return d;
  return place(d, p, to);
}

function place(d: Dock, p: PanelId, to: DropTarget): Dock {
  const out = clone(d);
  const from = findPanel(out, p);
  // The panel's own group, when it goes away: a new group "after" it takes its place.
  let gone: { zone: Zone; index: number; id: string } | null = null;
  if (from) {
    const g = from.group;
    g.panels = g.panels.filter((x) => x !== p);
    if (!g.panels.length) {
      gone = { zone: from.zone, index: out[from.zone].indexOf(g), id: g.id };
      out[from.zone] = out[from.zone].filter((x) => x !== g);
    } else if (g.active === p) g.active = g.panels[0];
  }
  const target = to.kind === "group" ? findGroup(out, to.id) : null;
  if (target) {
    target.group.panels.push(p);
    target.group.active = p;
    target.group.size = Math.max(target.group.size, MIN_WIDTH[p]);
    return out;
  }
  const g = group(p);
  const list = out[to.zone];
  if (to.zone === "left") list.unshift(g);
  else if (to.zone === "center") {
    const session = findPanel(out, "session");
    const after = (to.kind === "zone" && to.after) || (session?.zone === "center" ? session.group.id : null);
    const i = after ? list.findIndex((x) => x.id === after) : -1;
    if (i >= 0) list.splice(i + 1, 0, g);
    else if (after && gone?.id === after && gone.zone === "center") list.splice(gone.index, 0, g);
    else list.push(g);
  } else list.push(g);
  return out;
}

/** The panel a group shows: its active tab when that one is visible, else the first visible. */
export function shownPanel(g: DockGroup, visible: (p: PanelId) => boolean): PanelId | null {
  if (visible(g.active)) return g.active;
  return g.panels.find(visible) ?? null;
}
