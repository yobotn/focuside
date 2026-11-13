<script lang="ts">
  // A PDF in the viewer, one page at a time on a canvas. PDF.js loads on first use.
  import { CaretLeft, CaretRight, MagnifyingGlassMinus, MagnifyingGlassPlus } from "phosphor-svelte";
  import { errText } from "../api";
  import { formatSize } from "../editor";
  import { t } from "../i18n";
  import type { PdfDoc, RenderTask } from "../pdf";

  let { data }: { data: Uint8Array } = $props();

  let doc = $state.raw<PdfDoc | null>(null);
  let error = $state<string | null>(null);
  let page = $state(1);
  let pageInput = $state("1");
  let zoom = $state<number | "fit">("fit");
  let scale = $state(1);
  let stage: HTMLDivElement | undefined = $state();
  let canvas: HTMLCanvasElement | undefined = $state();
  let width = $state(0);

  const size = $derived(data.byteLength);

  $effect(() => {
    // PDF.js moves the buffer to its worker; give it a copy so `data` stays readable.
    const bytes = data.slice();
    let task: { destroy(): Promise<void> } | null = null;
    let gone = false;
    error = null;
    doc = null;
    page = 1;
    pageInput = "1";
    import("../pdf")
      .then(({ openPdf }) => {
        if (gone) return;
        const t2 = openPdf(bytes);
        task = t2;
        return t2.promise.then((d) => {
          if (!gone) doc = d;
        });
      })
      .catch((e) => {
        if (!gone) error = errText(e);
      });
    return () => {
      gone = true;
      task?.destroy();
    };
  });

  $effect(() => {
    if (!stage) return;
    const el = stage;
    const ro = new ResizeObserver(() => (width = el.clientWidth));
    ro.observe(el);
    const onwheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      setZoom(scale * Math.exp(-e.deltaY * 0.002));
    };
    el.addEventListener("wheel", onwheel, { passive: false });
    return () => {
      ro.disconnect();
      el.removeEventListener("wheel", onwheel);
    };
  });

  // Render the current page. A new render cancels the one still running.
  $effect(() => {
    const d = doc;
    const c = canvas;
    const n = page;
    const z = zoom;
    const w = width;
    if (!d || !c || w <= 0) return;
    let task: RenderTask | null = null;
    let gone = false;
    d.getPage(n)
      .then((p) => {
        if (gone) return;
        const base = p.getViewport({ scale: 1 });
        const s = z === "fit" ? Math.max(0.1, (w - 32) / base.width) : z;
        scale = s;
        const vp = p.getViewport({ scale: s });
        // Sharp on high-DPI screens, but keep the canvas under about 16 million pixels.
        const dpr = Math.min(window.devicePixelRatio || 1, Math.sqrt(16e6 / (vp.width * vp.height)));
        c.width = Math.floor(vp.width * dpr);
        c.height = Math.floor(vp.height * dpr);
        c.style.width = `${Math.floor(vp.width)}px`;
        c.style.height = `${Math.floor(vp.height)}px`;
        task = p.render({ canvas: c, viewport: vp, transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined });
        return task.promise;
      })
      .catch((e) => {
        if (!gone && (e as { name?: string })?.name !== "RenderingCancelledException") error = errText(e);
      });
    return () => {
      gone = true;
      task?.cancel();
    };
  });

  function setZoom(next: number) {
    zoom = Math.min(4, Math.max(0.1, next));
  }

  function go(n: number) {
    if (!doc) return;
    page = Math.min(doc.numPages, Math.max(1, n));
    pageInput = String(page);
    stage?.scrollTo({ top: 0 });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "PageDown" || e.key === "ArrowRight") go(page + 1);
    else if (e.key === "PageUp" || e.key === "ArrowLeft") go(page - 1);
    else if (e.key === "Home") go(1);
    else if (e.key === "End") go(doc?.numPages ?? 1);
    else if (e.key === "+" || e.key === "=") setZoom(scale * 1.25);
    else if (e.key === "-" || e.key === "_") setZoom(scale / 1.25);
    else if (e.key === "0") zoom = "fit";
    else if (e.key === "1") setZoom(1);
    else return;
    e.preventDefault();
  }
</script>

<div class="pdf-view">
  <div class="bar" role="toolbar" aria-label={t("media.pdf", { page })}>
    <button class="icon-btn" onclick={() => go(page - 1)} disabled={!doc || page <= 1} title="{t('media.prevPage')} (Page Up)" aria-label={t("media.prevPage")}><CaretLeft size={14} /></button>
    <label class="pages">
      <span class="sr-only">{t("media.page")}</span>
      <input
        class="page-input"
        inputmode="numeric"
        bind:value={pageInput}
        disabled={!doc}
        onchange={() => go(parseInt(pageInput, 10) || page)}
        onkeydown={(e) => {
          if (e.key === "Enter") go(parseInt(pageInput, 10) || page);
        }}
      />
      <span class="muted">{t("media.pageCount", { count: doc?.numPages ?? "-" })}</span>
    </label>
    <button class="icon-btn" onclick={() => go(page + 1)} disabled={!doc || page >= doc.numPages} title="{t('media.nextPage')} (Page Down)" aria-label={t("media.nextPage")}><CaretRight size={14} /></button>
    <span class="sep"></span>
    <button class="icon-btn" onclick={() => setZoom(scale / 1.25)} title="{t('media.zoomOut')} (-)" aria-label={t("media.zoomOut")}><MagnifyingGlassMinus size={14} /></button>
    <span class="pct" aria-live="polite">{Math.round(scale * 100)}%</span>
    <button class="icon-btn" onclick={() => setZoom(scale * 1.25)} title="{t('media.zoomIn')} (+)" aria-label={t("media.zoomIn")}><MagnifyingGlassPlus size={14} /></button>
    <button class="btn h-6" aria-pressed={zoom === "fit"} onclick={() => (zoom = "fit")} title="{t('media.fitWidth')} (0)">{t("media.fitWidth")}</button>
    <button class="btn h-6" aria-pressed={zoom === 1} onclick={() => setZoom(1)} title="{t('media.actualSize')} (1)" aria-label={t("media.actualSize")}>100%</button>
    <span class="flex-1"></span>
    <span class="muted truncate">{formatSize(size)}</span>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="stage" bind:this={stage} tabindex="0" role="group" aria-label="{t('media.pdf', { page })}. {t('media.zoomHelp')}" {onkeydown}>
    {#if error}
      <p class="m-auto p-4 text-danger selectable">{t("media.pdfFailed", { error })}</p>
    {:else if !doc}
      <p class="m-auto p-4 text-fg-muted">{t("common.loading")}</p>
    {/if}
    <canvas bind:this={canvas} style:display={doc && !error ? null : "none"}></canvas>
  </div>
</div>

<style>
  .pdf-view {
    flex: 1;
    min-height: 0;
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
    flex-wrap: wrap;
  }
  .pages {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .page-input {
    width: 3.5em;
    height: 24px;
    padding: 0 4px;
    text-align: center;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--fg-muted);
    font-size: 12px;
    min-width: 0;
  }
  .sep {
    width: 1px;
    height: 16px;
    background: var(--border);
    margin: 0 2px;
  }
  .pct {
    min-width: 3.5em;
    text-align: center;
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
  }
  .stage {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 16px;
    background: var(--bg-sunken);
  }
  .stage:focus-visible {
    outline-offset: -2px;
    border-radius: 0;
  }
  canvas {
    margin: 0 auto;
    flex-shrink: 0;
    background: #ffffff;
    box-shadow: 0 0 0 1px var(--border), 0 2px 8px rgb(0 0 0 / 0.12);
  }
</style>
