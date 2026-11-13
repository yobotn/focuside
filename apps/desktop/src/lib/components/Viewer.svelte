<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { ArrowCounterClockwise, CaretLeft, CaretRight, X } from "phosphor-svelte";
  import { api, errText } from "../api";
  import { formatSize } from "../editor";
  import { t } from "../i18n";
  import { keyLabel, mod } from "../platform";
  import { app, type Project } from "../stores/app.svelte";
  import type { FileVersions } from "../types";
  import { dock } from "../stores/dock.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import DockGrip from "./DockGrip.svelte";
  import FileIcon from "./FileIcon.svelte";
  import ImageView from "./ImageView.svelte";
  import Markdown from "./Markdown.svelte";
  import PdfView from "./PdfView.svelte";
  import { imageBlob, mediaKind, mediaLimit } from "../media";

  type Content =
    | { kind: "text"; content: string; size: number }
    | { kind: "binary"; size: number }
    | { kind: "large"; size: number }
    | { kind: "missing" };

  let { project }: { project: Project } = $props();

  const v = $derived(app.viewer!);
  const path = $derived(v.kind === "file" ? v.path : v.paths[v.index]);
  const isMd = $derived(/\.(md|markdown|mdx)$/i.test(path));
  const isSvg = $derived(/\.svg$/i.test(path));
  const media = $derived(mediaKind(path));

  let content = $state<Content | null>(null);
  let text = $state("");
  let saved = $state("");
  let error = $state<string | null>(null);
  let source = $state(false);
  let changedOnDisk = $state(false);
  let versions = $state<FileVersions | null>(null);
  let panel: HTMLElement | undefined = $state();
  // Image or PDF bytes for the file view, and both sides of a changed image in the diff view.
  let mediaData = $state.raw<{ path: string; buf: ArrayBuffer } | null>(null);
  let mediaError = $state<string | null>(null);
  let diffImages = $state.raw<{ before: Blob | null; after: Blob | null } | null>(null);

  const dirty = $derived(content?.kind === "text" && text !== saved);

  async function loadFile(p: string) {
    error = null;
    changedOnDisk = false;
    try {
      const c = await invoke<Content>("fs_read", { projectId: project.id, path: p });
      if (p !== path) return;
      content = c;
      if (c.kind === "text") {
        text = c.content;
        saved = c.content;
      }
      await loadMedia(p, c);
    } catch (e) {
      error = errText(e);
    }
  }

  /** Loads image or PDF bytes. SVG files that are text render from the text instead. */
  async function loadMedia(p: string, c: Content) {
    const kind = mediaKind(p);
    if (!kind || c.kind === "missing" || c.kind === "text" || c.size > mediaLimit(kind)) {
      if (mediaData?.path !== p) mediaData = null;
      return;
    }
    try {
      const buf = await invoke<ArrayBuffer>("fs_media", { projectId: project.id, path: p });
      if (p === path) {
        mediaError = null;
        mediaData = { path: p, buf };
      }
    } catch (e) {
      if (p === path) mediaError = errText(e);
    }
  }

  async function loadDiff(turnId: string, p: string) {
    error = null;
    versions = null;
    diffImages = null;
    try {
      const r = await api.fileVersions(project.id, turnId, p);
      if (r.binary && mediaKind(p) === "image") {
        const side = (after: boolean) =>
          invoke<ArrayBuffer>("turn_file_bytes", { projectId: project.id, turnId, path: p, after }).then(
            (b) => imageBlob(p, b),
            () => null,
          );
        const [before, after] = await Promise.all([side(false), side(true)]);
        if (p === path && (before || after)) diffImages = { before, after };
      }
      if (p === path) versions = r;
    } catch (e) {
      error = errText(e);
    }
  }

  $effect(() => {
    const cur = app.viewer;
    if (!cur) return;
    if (cur.kind === "file") {
      source = false;
      mediaError = null;
      loadFile(cur.path);
    } else {
      loadDiff(cur.turnId, cur.paths[cur.index]);
    }
  });

  // Changed on disk: reload, or ask when there are unsaved edits.
  $effect(() => {
    const id = project.id;
    const un = listen<{ projectId: string; paths: string[] }>("fs-changed", async (e) => {
      if (e.payload.projectId !== id || app.viewer?.kind !== "file") return;
      if (!e.payload.paths.includes(path)) return;
      const c = await invoke<Content>("fs_read", { projectId: id, path }).catch(() => null);
      if (!c) return;
      if (c.kind === "text" && c.content === saved) return;
      if (dirty) changedOnDisk = true;
      else {
        content = c;
        if (c.kind === "text") {
          text = c.content;
          saved = c.content;
        }
        loadMedia(path, c);
      }
    });
    return () => {
      un.then((f) => f());
    };
  });

  async function save() {
    if (!dirty) return;
    try {
      await invoke("fs_write", { projectId: project.id, path, content: text });
      saved = text;
      changedOnDisk = false;
      app.flash(t("viewer.saved"));
    } catch (e) {
      app.flash(t("viewer.saveFailed", { error: errText(e) }), "error");
    }
  }

  function close() {
    app.viewer = null;
  }

  function step(d: number) {
    const cur = app.viewer;
    if (cur?.kind !== "diff") return;
    const i = Math.max(0, Math.min(cur.paths.length - 1, cur.index + d));
    app.viewer = { ...cur, index: i };
  }

  async function revertFile() {
    const cur = app.viewer;
    if (cur?.kind !== "diff") return;
    try {
      const out = await api.revertFile(project.id, cur.turnId, cur.paths[cur.index]);
      app.flash(out.failed.length ? out.failed[0][1] : t("undo.revertFileDone", { path: cur.paths[cur.index] }), out.failed.length ? "error" : "muted");
    } catch (e) {
      app.flash(errText(e), "error");
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (v.kind !== "diff") return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "j" || e.key === "J") step(1);
    else if (e.key === "k" || e.key === "K") step(-1);
    else return;
    e.preventDefault();
  }

  function onwindowkey(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    const typing = !!el && (el.tagName === "TEXTAREA" || el.tagName === "INPUT" || el.isContentEditable);
    if (v.kind === "diff" && !typing && !app.overlay && !app.undoTarget) {
      onkeydown(e);
      if (e.defaultPrevented) return;
    }
    if (mod(e) && e.key.toLowerCase() === "s" && v.kind === "file") {
      e.preventDefault();
      save();
    }
  }

  const name = $derived(path.split("/").pop() ?? path);
  // An SVG that is text renders from the editor text, so unsaved edits show too.
  const svgBlob = $derived(isSvg && content?.kind === "text" ? imageBlob(path, text) : null);
  const mediaBlob = $derived(media === "image" && mediaData?.path === path ? imageBlob(path, mediaData.buf) : null);
  const pdfData = $derived(media === "pdf" && mediaData?.path === path ? new Uint8Array(mediaData.buf) : null);
  const mediaTooBig = $derived(!!media && !!content && content.kind !== "missing" && content.kind !== "text" && content.size > mediaLimit(media));
</script>

<svelte:window onkeydown={onwindowkey} />

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<section class="viewer" bind:this={panel} tabindex="-1" {onkeydown} aria-label={path}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="head" onpointerdown={(e) => dock.fromHeader("viewer", e)}>
    <FileIcon {path} size={15} />
    {#if v.kind === "diff"}
      <span class="text-fg-muted">{t("viewer.diff")}</span>
    {/if}
    <span class="truncate font-medium" title={path}>{path}</span>
    {#if dirty}<span class="dot" title={t("viewer.unsaved")}></span>{/if}
    <span class="flex-1"></span>
    {#if v.kind === "diff"}
      <span class="text-fg-muted">{t("viewer.diffOf", { index: v.index + 1, count: v.paths.length })}</span>
      <button class="icon-btn" onclick={() => step(-1)} disabled={v.index === 0} title="{t('viewer.prevFile')} (K)" aria-label={t("viewer.prevFile")}><CaretLeft size={14} /></button>
      <button class="icon-btn" onclick={() => step(1)} disabled={v.index >= v.paths.length - 1} title="{t('viewer.nextFile')} (J)" aria-label={t("viewer.nextFile")}><CaretRight size={14} /></button>
      <button class="btn h-6" onclick={revertFile} title={t("undo.revertFile")}><ArrowCounterClockwise size={13} />{t("undo.revertFile")}</button>
    {:else if (isMd || isSvg) && content?.kind === "text"}
      <button class="btn h-6" onclick={() => (source = !source)}>{source ? t("viewer.preview") : t("viewer.source")}</button>
    {/if}
    {#if v.kind === "file" && dirty}
      <button class="btn h-6" onclick={save} title={keyLabel("Mod+S")}>{t("common.save")}</button>
    {/if}
    <DockGrip panel="viewer" />
    <button class="icon-btn" onclick={close} title="{t('viewer.close')} (Esc)" aria-label={t("viewer.close")}><X size={14} /></button>
  </header>

  {#if changedOnDisk}
    <div class="banner">
      <span>{t("viewer.changedOnDisk")}</span>
      <button class="btn h-6" onclick={() => loadFile(path)}>{t("viewer.reload")}</button>
      <button class="btn h-6" onclick={() => (changedOnDisk = false)}>{t("viewer.keepMine")}</button>
    </div>
  {/if}

  <div class="body">
    {#if error}
      <p class="p-4 text-danger selectable">{error}</p>
    {:else if v.kind === "diff"}
      {#if !versions}
        <p class="p-4 text-fg-muted">{t("common.loading")}</p>
      {:else if versions.binary && diffImages}
        <div class="sides">
          {#each [{ label: t("viewer.before"), blob: diffImages.before }, { label: t("viewer.after"), blob: diffImages.after }] as side (side.label)}
            <div class="side">
              <p class="side-label">{side.label}</p>
              {#if side.blob}
                <ImageView blob={side.blob} label="{side.label}: {name}" compact />
              {:else}
                <p class="p-4 text-fg-muted">{t("viewer.noVersion")}</p>
              {/if}
            </div>
          {/each}
        </div>
      {:else if versions.binary}
        <p class="p-4 text-fg-muted">{t("viewer.binary", { size: "" })}</p>
      {:else if versions.after === null}
        <p class="px-4 pt-3 text-fg-muted">{t("viewer.deleted")}</p>
        <CodeEditor {path} text={versions.before ?? ""} readonly />
      {:else}
        <CodeEditor {path} text={versions.after} original={versions.before ?? ""} readonly />
      {/if}
    {:else if !content}
      <p class="p-4 text-fg-muted">{t("common.loading")}</p>
    {:else if content.kind === "missing"}
      <p class="p-4 text-fg-muted">{t("viewer.notFound", { path })}</p>
    {:else if !source && svgBlob}
      {#key path}<ImageView blob={svgBlob} label={name} />{/key}
    {:else if media && content.kind !== "text" && (mediaTooBig || mediaError)}
      <div class="p-4">
        <p class="mb-3 text-fg-muted selectable">
          {#if mediaError}
            {media === "pdf" ? t("media.pdfFailed", { error: mediaError }) : mediaError}
          {:else}
            {t(media === "pdf" ? "media.pdfTooLarge" : "media.imageTooLarge", { size: formatSize(content.size), limit: formatSize(mediaLimit(media)) })}
          {/if}
        </p>
        <button class="btn" onclick={() => openPath(project.root + "/" + path).catch((e) => app.flash(String(e), "error"))}>
          {t("viewer.openSystem")}
        </button>
      </div>
    {:else if mediaBlob}
      {#key path}<ImageView blob={mediaBlob} label={name} />{/key}
    {:else if pdfData}
      {#key path}<PdfView data={pdfData} />{/key}
    {:else if media && content.kind !== "text"}
      <p class="p-4 text-fg-muted">{t("common.loading")}</p>
    {:else if content.kind === "binary" || content.kind === "large"}
      <div class="p-4">
        <p class="mb-3 text-fg-muted">
          {content.kind === "binary" ? t("viewer.binary", { size: formatSize(content.size) }) : t("viewer.tooLarge", { size: formatSize(content.size) })}
        </p>
        <button class="btn" onclick={() => openPath(project.root + "/" + path).catch((e) => app.flash(String(e), "error"))}>
          {t("viewer.openSystem")}
        </button>
      </div>
    {:else if isMd && !source}
      <div class="md">
        <Markdown {text} doc={{ projectId: project.id, path, open: (p) => (app.viewer = { kind: "file", projectId: project.id, path: p }) }} />
      </div>
    {:else}
      <CodeEditor {path} {text} onchange={(t2) => (text = t2)} onsave={save} />
    {/if}
  </div>
  <span class="sr-only">{name}</span>
</section>

<style>
  .viewer {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    min-height: 0;
    outline: none;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-width: 0;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--fg-muted);
    flex-shrink: 0;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--warn-soft);
    color: var(--warn);
    border-bottom: 1px solid var(--border);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  .sides {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1px;
    background: var(--border);
  }
  .side {
    display: flex;
    flex-direction: column;
    min-height: 200px;
    min-width: 0;
    background: var(--bg);
  }
  .side-label {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 12px;
  }
  .md {
    padding: 20px 28px 40px;
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
  }
</style>
