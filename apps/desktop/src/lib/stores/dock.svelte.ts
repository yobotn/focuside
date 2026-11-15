// Panel docking in the UI: which panels are visible, moving them, and the drag that does it.
// Dragging uses pointer events, not HTML drag and drop: with the window's file drop turned on
// (Tauri dragDropEnabled), WebView2 does not deliver HTML drag events to the page.
import type { Attachment } from "svelte/attachments";
import { tick, untrack } from "svelte";
import { defaultDock, findPanel, isNoop, movePanel, normalizeDock, shownPanel, type DropTarget, type PanelId, type Zone } from "../dock";
import { app } from "./app.svelte";
import { search } from "./search.svelte";

/** A place the dragged panel can land, in window pixels. */
export interface DropZone {
  key: string;
  target: DropTarget;
  rect: { left: number; top: number; width: number; height: number };
  /** Edge zones show a label; groups do not. */
  edge: boolean;
}

const THRESHOLD = 5;

class DockStore {
  /** The panel being dragged, once the pointer moved past the threshold. */
  dragging = $state<PanelId | null>(null);
  pointer = $state({ x: 0, y: 0 });
  zones = $state<DropZone[]>([]);
  over = $state<DropZone | null>(null);
  /** Where each visible panel is drawn. Panels render once and move into their slot. */
  slots = $state<Partial<Record<PanelId, HTMLElement>>>({});

  visible(p: PanelId): boolean {
    const project = app.active;
    if (!project) return false;
    if (p === "session") return true;
    if (p === "side") return app.filesVisible;
    if (p === "search") return search.visible;
    if (p === "viewer") return !!app.viewer && app.viewer.projectId === project.id;
    if (p === "plan") return app.planVisible;
    if (p === "prompts") return app.promptsVisible;
    return app.terminalVisible;
  }

  /** True when the panel is visible and is the tab its group shows. */
  shown(p: PanelId): boolean {
    const at = findPanel(app.layout.dock, p);
    return !!at && shownPanel(at.group, (x) => this.visible(x)) === p;
  }

  /** Shows a panel and makes it the active tab of its group. The viewer needs an open file. */
  open(p: PanelId) {
    if (p === "side") app.filesVisible = true;
    else if (p === "search") search.visible = true;
    else if (p === "plan") app.planVisible = true;
    else if (p === "prompts") app.promptsVisible = true;
    else if (p === "terminal") app.terminalVisible = true;
    // A panel that is not in the dock goes back to its default place.
    if (!findPanel(app.layout.dock, p)) app.layout.dock = normalizeDock(app.layout.dock) ?? defaultDock();
    this.reveal(p);
  }

  /**
   * Shows a panel at its place in the dock (the last place it had, which is kept while it is
   * hidden, else its default place), brings it to the front of its group and moves the keyboard
   * focus into it. Returns false when the panel cannot show: no project is open, or the viewer
   * has no file.
   */
  async show(p: PanelId): Promise<boolean> {
    this.open(p);
    if (!this.visible(p)) return false;
    app.save();
    await tick();
    const slot = this.slots[p];
    if (!slot) return false;
    const el =
      slot.querySelector<HTMLElement>("[data-dock-focus]") ??
      slot.querySelector<HTMLElement>("textarea:not([disabled]), input:not([disabled]), [contenteditable=true]") ??
      slot.querySelector<HTMLElement>("button:not([disabled]), a[href], [tabindex]:not([tabindex='-1'])");
    if (el) el.focus();
    else {
      slot.tabIndex = -1;
      slot.focus();
    }
    return true;
  }

  /** Hides a panel. The viewer closes its file. The Session panel does not hide. */
  close(p: PanelId) {
    if (p === "session") return;
    if (p === "side") app.filesVisible = false;
    else if (p === "search") search.visible = false;
    else if (p === "plan") app.planVisible = false;
    else if (p === "prompts") app.promptsVisible = false;
    else if (p === "terminal") app.terminalVisible = false;
    else app.viewer = null;
    app.save();
  }

  /** Mod+B, Mod+` and the palette: a panel behind another tab comes to the front, else it toggles. */
  toggle(p: PanelId) {
    if (this.visible(p) && !this.shown(p)) this.reveal(p);
    else if (this.visible(p)) this.close(p);
    else this.open(p);
    app.save();
  }

  /** Makes the panel the active tab of its group. */
  reveal(p: PanelId) {
    const at = findPanel(app.layout.dock, p);
    if (at && at.group.active !== p) {
      at.group.active = p;
      app.save();
    }
  }

  zoneOf(p: PanelId): Zone | null {
    return findPanel(app.layout.dock, p)?.zone ?? null;
  }

  move(p: PanelId, to: DropTarget) {
    if (isNoop(app.layout.dock, p, to)) return;
    app.layout.dock = movePanel(app.layout.dock, p, to);
    app.save();
  }

  /** Pointer down on a header or tab. A drag starts once the pointer moves a few pixels. */
  start(p: PanelId, e: PointerEvent) {
    if (e.button !== 0) return;
    const x0 = e.clientX;
    const y0 = e.clientY;
    const move = (ev: PointerEvent) => {
      if (!this.dragging) {
        if (Math.abs(ev.clientX - x0) + Math.abs(ev.clientY - y0) < THRESHOLD) return;
        this.begin(p);
      }
      this.pointer = { x: ev.clientX, y: ev.clientY };
      this.over = this.hit(ev.clientX, ev.clientY);
      ev.preventDefault();
    };
    const end = (drop: boolean) => {
      window.removeEventListener("pointermove", move, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("pointercancel", cancel, true);
      window.removeEventListener("keydown", key, true);
      if (!this.dragging) return;
      const over = this.over;
      this.dragging = null;
      this.over = null;
      this.zones = [];
      document.body.classList.remove("docking");
      // The click that ends a drag must not also press the button under the pointer.
      const swallow = (ev: Event) => {
        ev.stopPropagation();
        ev.preventDefault();
      };
      window.addEventListener("click", swallow, { capture: true, once: true });
      setTimeout(() => window.removeEventListener("click", swallow, true), 0);
      if (drop && over) this.move(p, over.target);
    };
    const up = () => end(true);
    const cancel = () => end(false);
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape" || !this.dragging) return;
      ev.preventDefault();
      ev.stopPropagation();
      end(false);
    };
    window.addEventListener("pointermove", move, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("pointercancel", cancel, true);
    window.addEventListener("keydown", key, true);
  }

  /** Pointer down on a panel header: drags unless it started on a control in the header. */
  fromHeader(p: PanelId, e: PointerEvent) {
    const el = e.target as Element | null;
    if (el?.closest("button, input, select, textarea, a, [contenteditable]")) return;
    this.start(p, e);
  }

  private begin(p: PanelId) {
    this.dragging = p;
    this.zones = this.measure(p);
    document.body.classList.add("docking");
  }

  /** The drop zones, from the work area and the groups on screen. */
  private measure(p: PanelId): DropZone[] {
    const work = document.querySelector<HTMLElement>("[data-dock-work]")?.getBoundingClientRect();
    const fill = document.querySelector<HTMLElement>("[data-dock-fill]");
    const center = fill?.getBoundingClientRect();
    const top = document.querySelector<HTMLElement>("[data-dock-top]")?.getBoundingClientRect();
    if (!work || !fill || !center || !top) return [];
    const fillId = fill.dataset.dockFill || undefined;
    const d = app.layout.dock;
    const out: DropZone[] = [];
    const edge = 64;
    const add = (key: string, target: DropTarget, r: DropZone["rect"], isEdge: boolean) => {
      if (!isNoop(d, p, target)) out.push({ key, target, rect: r, edge: isEdge });
    };
    // Edge zones first: they win where they overlap a group.
    add("left", { kind: "zone", zone: "left" }, { left: work.left, top: work.top, width: edge, height: work.height }, true);
    add("right", { kind: "zone", zone: "right" }, { left: work.right - edge, top: work.top, width: edge, height: work.height }, true);
    add("bottom", { kind: "zone", zone: "bottom" }, { left: top.left, top: top.bottom - edge, width: top.width, height: edge }, true);
    // The center: a new group right of the group that fills the middle (its right part; the
    // rest of that group takes the panel as a tab), or the whole empty middle.
    const cw = fillId ? Math.max(80, Math.round(center.width * 0.35)) : center.width;
    add(
      "center",
      { kind: "zone", zone: "center", after: fillId },
      { left: center.right - cw, top: center.top, width: cw, height: center.height },
      false,
    );
    for (const el of document.querySelectorAll<HTMLElement>("[data-dock-group]")) {
      const id = el.dataset.dockGroup!;
      const zone = el.dataset.dockZone as Zone;
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) continue;
      add(`group-${id}`, { kind: "group", zone, id }, { left: r.left, top: r.top, width: r.width, height: r.height }, false);
    }
    return out;
  }

  private hit(x: number, y: number): DropZone | null {
    for (const z of this.zones) {
      const r = z.rect;
      if (x >= r.left && x < r.left + r.width && y >= r.top && y < r.top + r.height) return z;
    }
    return null;
  }
}

export const dock = new DockStore();

/** Marks the element where a panel is drawn. */
export function dockSlot(p: PanelId): Attachment<HTMLElement> {
  return (node) => {
    dock.slots[p] = node;
    return () => {
      untrack(() => {
        if (dock.slots[p] === node) delete dock.slots[p];
      });
    };
  };
}

/** Moves a panel's element into its slot, and again whenever the slot changes. */
export function dockHost(p: PanelId): Attachment<HTMLElement> {
  return (node) => {
    const slot = dock.slots[p];
    if (slot && node.parentElement !== slot) slot.appendChild(node);
  };
}
