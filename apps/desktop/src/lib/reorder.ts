// Drag to reorder the items of a list (session tabs). Items carry `data-key`. A line shows where
// the item lands. Pointer events, not HTML drag and drop (see stores/dock.svelte.ts).
import type { Attachment } from "svelte/attachments";

const THRESHOLD = 5;

export function reorder(axis: "x" | "y", onmove: (key: string, index: number) => void): Attachment<HTMLElement> {
  return (list) => {
    const down = (e: PointerEvent) => {
      if (e.button !== 0) return;
      const target = e.target as Element;
      if (target.closest("input, .close")) return;
      const item = target.closest<HTMLElement>("[data-key]");
      if (!item || !list.contains(item)) return;
      const key = item.dataset.key!;
      const x0 = e.clientX;
      const y0 = e.clientY;
      let marker: HTMLDivElement | null = null;
      let index = -1;

      const items = () => [...list.querySelectorAll<HTMLElement>("[data-key]")];
      const move = (ev: PointerEvent) => {
        if (!marker) {
          if (Math.abs(ev.clientX - x0) + Math.abs(ev.clientY - y0) < THRESHOLD) return;
          marker = document.createElement("div");
          marker.className = `reorder-marker ${axis}`;
          document.body.appendChild(marker);
          item.classList.add("reordering");
          document.body.classList.add("docking");
        }
        ev.preventDefault();
        const all = items();
        const at = axis === "x" ? ev.clientX : ev.clientY;
        index = all.filter((el) => {
          const r = el.getBoundingClientRect();
          return (axis === "x" ? r.left + r.width / 2 : r.top + r.height / 2) < at;
        }).length;
        const box = list.getBoundingClientRect();
        const ref = all[Math.min(index, all.length - 1)].getBoundingClientRect();
        const before = index < all.length;
        if (axis === "x") {
          const x = before ? ref.left - 1 : ref.right + 1;
          Object.assign(marker.style, { left: `${x - 1}px`, top: `${ref.top}px`, height: `${ref.height}px` });
        } else {
          const y = before ? ref.top - 1 : ref.bottom + 1;
          Object.assign(marker.style, { top: `${y - 1}px`, left: `${Math.max(box.left, ref.left)}px`, width: `${ref.width}px` });
        }
      };
      const end = (drop: boolean) => {
        window.removeEventListener("pointermove", move, true);
        window.removeEventListener("pointerup", up, true);
        window.removeEventListener("pointercancel", cancel, true);
        window.removeEventListener("keydown", esc, true);
        if (!marker) return;
        marker.remove();
        item.classList.remove("reordering");
        document.body.classList.remove("docking");
        const swallow = (ev: Event) => {
          ev.stopPropagation();
          ev.preventDefault();
        };
        window.addEventListener("click", swallow, { capture: true, once: true });
        setTimeout(() => window.removeEventListener("click", swallow, true), 0);
        if (!drop) return;
        const from = items().indexOf(item);
        const to = index > from ? index - 1 : index;
        if (from >= 0 && to !== from) onmove(key, to);
      };
      const up = () => end(true);
      const cancel = () => end(false);
      const esc = (ev: KeyboardEvent) => {
        if (ev.key !== "Escape" || !marker) return;
        ev.preventDefault();
        ev.stopPropagation();
        end(false);
      };
      window.addEventListener("keydown", esc, true);
      window.addEventListener("pointermove", move, true);
      window.addEventListener("pointerup", up, true);
      window.addEventListener("pointercancel", cancel, true);
    };
    list.addEventListener("pointerdown", down);
    return () => list.removeEventListener("pointerdown", down);
  };
}
