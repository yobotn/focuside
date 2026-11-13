// The panel layout around the session area. Four dock zones hold groups, and a group holds one
// or more panels as tabs. Left and right run the full height. Center groups sit beside the
// session area (the editor area, where the viewer and the plan open). Bottom runs under the
// session area and the center groups. The session area itself always stays in the center.
// Pure functions only, so the layout can be checked and changed without the UI.

export type PanelId = "side" | "viewer" | "plan" | "terminal";
export type Zone = "left" | "center" | "right" | "bottom";

export const PANELS: PanelId[] = ["side", "viewer", "plan", "terminal"];
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

/** Where a dragged panel lands: a new group at the outer edge of a zone, or a tab in a group. */
export type DropTarget = { kind: "zone"; zone: Zone } | { kind: "group"; zone: Zone; id: string };

export const MIN_WIDTH: Record<PanelId, number> = { side: 160, viewer: 320, plan: 280, terminal: 240 };
export const DEFAULT_WIDTH: Record<PanelId, number> = { side: 248, viewer: 640, plan: 640, terminal: 480 };
export const DEFAULT_BOTTOM = 260;

let seq = 0;
export function groupId(): string {
  seq += 1;
  return `g${Date.now().toString(36)}${seq}`;
}

function group(p: PanelId, size = DEFAULT_WIDTH[p]): DockGroup {
  return { id: groupId(), panels: [p], active: p, size };
}

/** Side bar on the left, the viewer and the plan as tabs in the editor area, terminal below. */
export function defaultDock(): Dock {
  const editor: DockGroup = { id: groupId(), panels: ["viewer", "plan"], active: "viewer", size: DEFAULT_WIDTH.viewer };
  return { left: [group("side")], center: [editor], right: [], bottom: [group("terminal")] };
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
  return d;
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
  // its default place: a tab next to a panel of its default group, or a new group.
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

/** New groups go to the outer edge (left, right), next to the session area (center), or last. */
function atStart(zone: Zone): boolean {
  return zone === "left" || zone === "center";
}

function clone(d: Dock): Dock {
  return {
    left: d.left.map((g) => ({ ...g, panels: [...g.panels] })),
    center: d.center.map((g) => ({ ...g, panels: [...g.panels] })),
    right: d.right.map((g) => ({ ...g, panels: [...g.panels] })),
    bottom: d.bottom.map((g) => ({ ...g, panels: [...g.panels] })),
  };
}

/** True when the drop would leave the layout as it is. */
export function isNoop(d: Dock, p: PanelId, to: DropTarget): boolean {
  const from = findPanel(d, p);
  if (!from) return false;
  if (to.kind === "group") return to.id === from.group.id;
  if (from.zone !== to.zone || from.group.panels.length > 1) return false;
  // Alone in a group: a no-op when it already sits where a new group would go.
  const list = d[to.zone];
  const outer = atStart(to.zone) ? list[0] : list[list.length - 1];
  return outer === from.group;
}

/** Moves a panel to a drop target and returns the new dock. The panel becomes the active tab. */
export function movePanel(d: Dock, p: PanelId, to: DropTarget): Dock {
  if (isNoop(d, p, to)) return d;
  const out = clone(d);
  const from = findPanel(out, p);
  if (from) {
    const g = from.group;
    g.panels = g.panels.filter((x) => x !== p);
    if (!g.panels.length) out[from.zone] = out[from.zone].filter((x) => x !== g);
    else if (g.active === p) g.active = g.panels[0];
  }
  const target = to.kind === "group" ? findGroup(out, to.id) : null;
  if (target) {
    target.group.panels.push(p);
    target.group.active = p;
    target.group.size = Math.max(target.group.size, MIN_WIDTH[p]);
    return out;
  }
  const g = group(p);
  if (atStart(to.zone)) out[to.zone].unshift(g);
  else out[to.zone].push(g);
  return out;
}

/** The panel a group shows: its active tab when that one is visible, else the first visible. */
export function shownPanel(g: DockGroup, visible: (p: PanelId) => boolean): PanelId | null {
  if (visible(g.active)) return g.active;
  return g.panels.find(visible) ?? null;
}
