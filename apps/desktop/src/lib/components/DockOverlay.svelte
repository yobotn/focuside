<script lang="ts">
  // Shown while a panel is dragged: every drop zone, the zone under the pointer, a preview of
  // where the panel lands, and the panel name next to the pointer. It takes no pointer events;
  // the drag itself listens on the window (stores/dock).
  import { DEFAULT_WIDTH } from "../dock";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { dock, type DropZone } from "../stores/dock.svelte";
  import { panelName, zoneName } from "./dockNames";

  const p = $derived(dock.dragging!);

  function preview(z: DropZone): DropZone["rect"] {
    const r = z.rect;
    // The center zone is already the part of the middle where the new group goes.
    if (z.target.kind === "group" || z.target.zone === "center") return r;
    const w = Math.min(DEFAULT_WIDTH[p], window.innerWidth * 0.35);
    if (z.target.zone === "left") return { left: r.left, top: r.top, width: w, height: r.height };
    if (z.target.zone === "right") return { left: r.left + r.width - w, top: r.top, width: w, height: r.height };
    const h = Math.min(app.layout.bottomHeight, window.innerHeight * 0.4);
    return { left: r.left, top: r.top + r.height - h, width: r.width, height: h };
  }

  const box = (r: DropZone["rect"]) => `left: ${r.left}px; top: ${r.top}px; width: ${r.width}px; height: ${r.height}px`;
  const label = (z: DropZone) => (z.target.kind === "group" ? t("dock.asTab") : zoneName(z.target.zone));
</script>

<div class="overlay" aria-hidden="true">
  {#each dock.zones as z (z.key)}
    <div class="zone" class:edge={z.edge} class:flat={z.target.kind === "zone" && z.target.zone === "bottom"} class:over={dock.over?.key === z.key} style={box(z.rect)}>
      {#if z.edge && z.target.kind === "zone" && z.target.zone !== "center"}<span class="tag">{label(z)}</span>{/if}
    </div>
  {/each}
  {#if dock.over}
    <div class="preview" style={box(preview(dock.over))}>
      <span class="tag">{label(dock.over)}</span>
    </div>
  {/if}
  <div class="ghost" style="left: {dock.pointer.x + 14}px; top: {dock.pointer.y + 12}px">{panelName(p)}</div>
</div>
<p class="sr-only" role="status">{dock.over ? t("dock.dropHere", { panel: panelName(p), place: label(dock.over) }) : ""}</p>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    pointer-events: none;
  }
  .zone {
    position: fixed;
    border-radius: 6px;
  }
  .zone.edge {
    display: grid;
    place-items: center;
    border: 1px dashed color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }
  .zone:not(.edge) {
    outline: 1px dashed var(--border-strong);
    outline-offset: -3px;
  }
  .zone.over {
    border-color: var(--accent);
    outline-color: var(--accent);
  }
  .preview {
    position: fixed;
    display: grid;
    place-items: center;
    border: 2px solid var(--accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    transition:
      left var(--fast) var(--ease),
      top var(--fast) var(--ease),
      width var(--fast) var(--ease),
      height var(--fast) var(--ease);
  }
  .tag {
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-size: 12px;
    font-weight: 600;
    box-shadow: 0 0 0 1px var(--border);
    white-space: nowrap;
  }
  .zone.edge .tag {
    writing-mode: vertical-rl;
    padding: 8px 2px;
  }
  .zone.flat .tag {
    writing-mode: horizontal-tb;
    padding: 2px 8px;
  }
  .ghost {
    position: fixed;
    padding: 3px 10px;
    border-radius: 6px;
    background: var(--fg);
    color: var(--bg);
    font-size: 12px;
    font-weight: 600;
    box-shadow: var(--shadow);
    white-space: nowrap;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
