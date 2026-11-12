<script lang="ts">
  // A drag handle between two panels. `edge` is the panel's edge the handle sits on: a panel on
  // the left of the conversation has its handle on its right edge, and the other way round.
  // Arrow keys resize too; double-click restores the default size.
  import { t } from "../i18n";

  let {
    edge,
    value,
    min,
    max,
    onchange,
    onreset,
  }: {
    edge: "left" | "right" | "top";
    value: number;
    min: number;
    max: () => number;
    onchange: (v: number, done: boolean) => void;
    onreset: () => void;
  } = $props();

  let dragging = $state(false);

  const clamp = (v: number) => Math.round(Math.max(min, Math.min(max(), v)));

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const start = edge === "top" ? e.clientY : e.clientX;
    const startV = value;
    dragging = true;
    const sign = edge === "left" || edge === "top" ? -1 : 1;
    const at = (ev: PointerEvent) => clamp(startV + sign * ((edge === "top" ? ev.clientY : ev.clientX) - start));
    const move = (ev: PointerEvent) => onchange(at(ev), false);
    const up = (ev: PointerEvent) => {
      dragging = false;
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      onchange(at(ev), true);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  }

  function onkeydown(e: KeyboardEvent) {
    const grow = edge === "top" ? "ArrowUp" : edge === "right" ? "ArrowRight" : "ArrowLeft";
    const shrink = edge === "top" ? "ArrowDown" : edge === "right" ? "ArrowLeft" : "ArrowRight";
    const stepSize = e.shiftKey ? 64 : 16;
    if (e.key === grow) onchange(clamp(value + stepSize), true);
    else if (e.key === shrink) onchange(clamp(value - stepSize), true);
    else if (e.key === "Home") onreset();
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="split {edge}"
  class:dragging
  role="separator"
  aria-orientation={edge === "top" ? "horizontal" : "vertical"}
  aria-valuenow={value}
  aria-valuemin={min}
  aria-label={t("layout.resize")}
  title={t("layout.resizeHint")}
  tabindex="0"
  onpointerdown={down}
  ondblclick={onreset}
  {onkeydown}
></div>

<style>
  .split {
    position: relative;
    flex-shrink: 0;
    background: var(--border);
    z-index: 5;
    outline: none;
  }
  .split.left,
  .split.right {
    width: 1px;
    cursor: col-resize;
  }
  .split.top {
    height: 1px;
    cursor: row-resize;
  }
  /* A wider invisible hit area around the 1px line. */
  .split::after {
    content: "";
    position: absolute;
    inset: 0 -4px;
  }
  .split.top::after {
    inset: -4px 0;
  }
  .split:hover,
  .split:focus-visible,
  .split.dragging {
    background: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
</style>
