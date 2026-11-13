<script lang="ts">
  // One image in the viewer: fitted by default, zoom with buttons, Ctrl+wheel or + and - keys,
  // and a checkerboard behind transparent pixels. The image is always an img element with a
  // blob URL, so SVG files never become DOM and their scripts never run.
  import { MagnifyingGlassMinus, MagnifyingGlassPlus } from "phosphor-svelte";
  import { tick } from "svelte";
  import { formatSize } from "../editor";
  import { t } from "../i18n";

  let { blob, label, compact = false }: { blob: Blob; label: string; compact?: boolean } = $props();

  let url = $state<string | null>(null);
  let natural = $state<{ w: number; h: number } | null>(null);
  let failed = $state(false);
  let zoom = $state<number | "fit">("fit");
  let stage: HTMLDivElement | undefined = $state();
  let box = $state({ w: 0, h: 0 });

  $effect(() => {
    const u = URL.createObjectURL(blob);
    url = u;
    failed = false;
    natural = null;
    return () => URL.revokeObjectURL(u);
  });

  // SVG files without a size report 0; show them at a default size.
  const dims = $derived(natural && natural.w > 0 && natural.h > 0 ? natural : { w: 300, h: 150 });
  const fitScale = $derived(box.w > 0 && box.h > 0 ? Math.min(1, (box.w - 32) / dims.w, (box.h - 32) / dims.h) : 1);
  const scale = $derived(Math.max(0.01, zoom === "fit" ? fitScale : zoom));

  $effect(() => {
    if (!stage) return;
    const el = stage;
    const ro = new ResizeObserver(() => (box = { w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    const onwheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      zoomTo(scale * Math.exp(-e.deltaY * 0.002), e);
    };
    el.addEventListener("wheel", onwheel, { passive: false });
    return () => {
      ro.disconnect();
      el.removeEventListener("wheel", onwheel);
    };
  });

  /** Sets the zoom and keeps the point under the mouse (or the center) in place. */
  async function zoomTo(next: number, at?: { clientX: number; clientY: number }) {
    const el = stage;
    const s = Math.min(32, Math.max(0.02, next));
    if (!el) {
      zoom = s;
      return;
    }
    const r = el.getBoundingClientRect();
    const ox = at ? at.clientX - r.left : el.clientWidth / 2;
    const oy = at ? at.clientY - r.top : el.clientHeight / 2;
    const fx = (el.scrollLeft + ox) / Math.max(1, el.scrollWidth);
    const fy = (el.scrollTop + oy) / Math.max(1, el.scrollHeight);
    zoom = s;
    await tick();
    el.scrollLeft = fx * el.scrollWidth - ox;
    el.scrollTop = fy * el.scrollHeight - oy;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "+" || e.key === "=") zoomTo(scale * 1.25);
    else if (e.key === "-" || e.key === "_") zoomTo(scale / 1.25);
    else if (e.key === "0") zoom = "fit";
    else if (e.key === "1") zoomTo(1);
    else return;
    e.preventDefault();
  }

  function onload(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    natural = { w: img.naturalWidth, h: img.naturalHeight };
  }

  const info = $derived(
    natural && natural.w > 0 ? t("media.imageInfo", { width: natural.w, height: natural.h, size: formatSize(blob.size) }) : formatSize(blob.size),
  );
</script>

<div class="image-view" class:compact>
  {#if !compact}
    <div class="bar" role="toolbar" aria-label={label}>
      <button class="icon-btn" onclick={() => zoomTo(scale / 1.25)} title="{t('media.zoomOut')} (-)" aria-label={t("media.zoomOut")}><MagnifyingGlassMinus size={14} /></button>
      <span class="pct" aria-live="polite">{Math.round(scale * 100)}%</span>
      <button class="icon-btn" onclick={() => zoomTo(scale * 1.25)} title="{t('media.zoomIn')} (+)" aria-label={t("media.zoomIn")}><MagnifyingGlassPlus size={14} /></button>
      <button class="btn h-6" aria-pressed={zoom === "fit"} onclick={() => (zoom = "fit")} title="{t('media.fitHint')} (0)">{t("media.fit")}</button>
      <button class="btn h-6" aria-pressed={zoom === 1} onclick={() => zoomTo(1)} title="{t('media.actualSize')} (1)" aria-label={t("media.actualSize")}>100%</button>
      <span class="flex-1"></span>
      <span class="info truncate selectable">{info}</span>
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="stage" bind:this={stage} tabindex="0" role="group" aria-label="{label}. {t('media.zoomHelp')}" {onkeydown}>
    {#if failed}
      <p class="m-auto p-4 text-fg-muted">{t("media.imageFailed")}</p>
    {:else if url}
      <img
        src={url}
        alt={label}
        draggable="false"
        class:pixelated={scale >= 3}
        style:width="{dims.w * scale}px"
        style:height="{dims.h * scale}px"
        {onload}
        onerror={() => (failed = true)}
      />
    {/if}
  </div>
  {#if compact}
    <p class="info px-2 py-1 selectable">{info}</p>
  {/if}
</div>

<style>
  .image-view {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-width: 0;
  }
  .pct {
    min-width: 3.5em;
    text-align: center;
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
  }
  .info {
    color: var(--fg-muted);
    font-size: 12px;
    min-width: 0;
  }
  .stage {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    padding: 16px;
  }
  .stage:focus-visible {
    outline-offset: -2px;
    border-radius: 0;
  }
  img {
    margin: auto;
    flex-shrink: 0;
    max-width: none;
    --check: color-mix(in srgb, var(--fg-muted) 22%, transparent);
    background-color: var(--bg);
    background-image: conic-gradient(var(--check) 25%, transparent 0 50%, var(--check) 0 75%, transparent 0);
    background-size: 16px 16px;
    box-shadow: 0 0 0 1px var(--border);
  }
  img.pixelated {
    image-rendering: pixelated;
  }
</style>
