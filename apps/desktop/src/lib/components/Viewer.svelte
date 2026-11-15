<script lang="ts">
  // The viewer: editor tabs (stores/editor.svelte.ts), breadcrumbs, and the open file in
  // CodeMirror, rendered Markdown, an image or a PDF. The diff tab shows the review of a turn's
  // changes (ReviewView). Ctrl+Page Up and Page Down switch tabs while the focus is in the viewer.
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { X } from "phosphor-svelte";
  import { errText } from "../api";
  import { formatSize } from "../editor";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app, type Project } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";
  import { editor } from "../stores/editor.svelte";
  import Breadcrumbs from "./Breadcrumbs.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import DockGrip from "./DockGrip.svelte";
  import EditorTabs from "./EditorTabs.svelte";
  import ImageView from "./ImageView.svelte";
  import Markdown from "./Markdown.svelte";
  import PdfView from "./PdfView.svelte";
  import ReviewView from "./ReviewView.svelte";
  import { imageBlob, mediaKind, mediaLimit } from "../media";

  let { project }: { project: Project } = $props();

  const w = $derived(editor.get(project.id));
  const tab = $derived(w?.current ?? null);
  const fileTab = $derived(tab?.kind === "file" ? tab : null);
  const diffTab = $derived(tab?.kind === "diff" ? tab : null);
  const doc = $derived(fileTab && w ? w.docs[fileTab.path] : undefined);
  const path = $derived(fileTab ? fileTab.path : "");
  const content = $derived(doc?.content ?? null);
  const isMd = $derived(/\.(md|markdown|mdx)$/i.test(path));
  const isSvg = $derived(/\.svg$/i.test(path));
  const media = $derived(mediaKind(path));

  /** Markdown and SVG files show rendered; this set holds the files switched to source. */
  let sourceOf = $state<Record<string, boolean>>({});
  const source = $derived(!!sourceOf[path]);
  // Image or PDF bytes for the file view.
  let mediaData = $state.raw<{ path: string; rev: number; buf: ArrayBuffer } | null>(null);
  let mediaError = $state<string | null>(null);

  const dirty = $derived(!!doc?.dirty);

  // A jump to a line (a search result, Quick open) in Markdown or SVG shows the source.
  $effect(() => {
    const j = editor.jump;
    if (j && j.projectId === project.id && /\.(md|markdown|mdx|svg)$/i.test(j.path)) sourceOf[j.path] = true;
  });

  /** Loads image or PDF bytes. SVG files that are text render from the text instead. */
  $effect(() => {
    const p = path;
    const c = content;
    const rev = doc?.rev ?? 0;
    if (!fileTab) return;
    const kind = mediaKind(p);
    if (!kind || !c || c.kind === "missing" || c.kind === "text" || c.size > mediaLimit(kind)) return;
    if (mediaData?.path === p && mediaData.rev === rev) return;
    mediaError = null;
    invoke<ArrayBuffer>("fs_media", { projectId: project.id, path: p })
      .then((buf) => {
        if (p === path) mediaData = { path: p, rev, buf };
      })
      .catch((e) => {
        if (p === path) mediaError = errText(e);
      });
  });

  function onkeydown(e: KeyboardEvent) {
    // Ctrl+Page Up and Page Down: the previous or next editor tab (Ctrl on every OS, as in
    // VS Code). Ctrl+Shift+Page Up and Page Down stay with the session tabs. The review
    // (ReviewView) has its own keys.
    if (e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey && (e.key === "PageUp" || e.key === "PageDown") && w) {
      e.preventDefault();
      e.stopPropagation();
      editor.cycle(w, e.key === "PageUp" ? -1 : 1);
    }
  }

  function close() {
    if (w) editor.hide(w);
  }

  function pinActive() {
    if (w && fileTab?.preview) editor.pin(w, fileTab.path);
  }

  const name = $derived(path.split("/").pop() ?? path);
  // An SVG that is text renders from the editor text, so unsaved edits show too.
  const svgBlob = $derived(isSvg && content?.kind === "text" && doc ? imageBlob(path, doc.text) : null);
  const mediaBlob = $derived(media === "image" && mediaData?.path === path ? imageBlob(path, mediaData.buf) : null);
  const pdfData = $derived(media === "pdf" && mediaData?.path === path ? new Uint8Array(mediaData.buf) : null);
  const mediaTooBig = $derived(!!media && !!content && content.kind !== "missing" && content.kind !== "text" && content.size > mediaLimit(media));
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<section class="viewer" tabindex="-1" {onkeydown} aria-label={path ? t("editor.label", { path }) : t("dock.viewer")}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="head" onpointerdown={(e) => dock.fromHeader("viewer", e)}>
    {#if w}<EditorTabs {project} {w} />{/if}
    <span class="grab" title={t("dock.tabHint")}></span>
    <DockGrip panel="viewer" />
    <button class="icon-btn" onclick={close} title="{t('viewer.close')} (Esc)" aria-label={t("viewer.close")}><X size={14} /></button>
  </header>

  {#if fileTab}
    <div class="bar">
      <Breadcrumbs projectId={project.id} {path} />
      <span class="flex-1"></span>
      {#if (isMd || isSvg) && content?.kind === "text"}
        <button class="btn h-6" onclick={() => (sourceOf[path] = !source)}>{source ? t("viewer.preview") : t("viewer.source")}</button>
      {/if}
      {#if dirty}
        <button class="btn h-6" onclick={() => editor.saveActive()} title={keyLabel("Mod+S")}>{t("common.save")}</button>
      {/if}
    </div>
  {/if}

  {#if doc?.disk && w}
    <div class="banner" role="alert">
      <span class="flex-1">{doc.disk === "deleted" ? t("viewer.deletedOnDisk") : t("viewer.changedOnDisk")}</span>
      <button class="btn h-6" onclick={() => w && doc && editor.keepMine(w, doc)}>{t("viewer.keepMine")}</button>
      {#if doc.disk === "deleted"}
        <button
          class="btn h-6"
          onclick={() => {
            if (!w || !doc) return;
            doc.text = doc.saved;
            editor.close(w, [doc.path]);
          }}>{t("viewer.closeFile")}</button
        >
      {:else}
        <button class="btn h-6" onclick={() => w && doc && editor.reload(w, doc)}>{t("viewer.loadFromDisk")}</button>
      {/if}
    </div>
  {/if}

  <div class="body">
    {#if diffTab}
      <ReviewView {project} tab={diffTab} />
    {:else if !doc}
      <p class="p-4 text-fg-muted">{t("viewer.noFile")}</p>
    {:else if doc.error}
      <p class="p-4 text-danger selectable">{doc.error}</p>
    {:else if !content}
      <p class="p-4 text-fg-muted">{t("common.loading")}</p>
    {:else if content.kind === "missing" && !doc.disk}
      <p class="p-4 text-fg-muted">{t("viewer.notFound", { path })}</p>
    {:else if !source && svgBlob}
      {#key path}<ImageView blob={svgBlob} label={name} />{/key}
    {:else if media && content.kind !== "text" && content.kind !== "missing" && (mediaTooBig || mediaError)}
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
    {:else if mediaBlob && content.kind !== "text"}
      {#key path}<ImageView blob={mediaBlob} label={name} />{/key}
    {:else if pdfData && content.kind !== "text"}
      {#key path}<PdfView data={pdfData} />{/key}
    {:else if media && content.kind !== "text" && content.kind !== "missing"}
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
        <Markdown text={doc.text} doc={{ projectId: project.id, path, open: (p) => editor.open(project.id, p) }} />
      </div>
    {:else}
      <CodeEditor {doc} projectId={project.id} onsave={() => editor.saveActive()} onedit={pinActive} />
    {/if}
  </div>
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
    align-items: stretch;
    height: 36px;
    padding-inline-end: 6px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
    flex-shrink: 0;
    min-width: 0;
  }
  .head > :global(.icon-btn),
  .head > :global(.grip) {
    align-self: center;
  }
  .grab {
    flex: 1;
    min-width: 24px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px 0 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-width: 0;
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
  .md {
    padding: 20px 28px 40px;
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
  }
</style>
