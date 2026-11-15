<script lang="ts">
  // The review in the diff tab: every file of a turn (or of every turn of the session) stacked,
  // each with its hunks. Undo change puts one hunk back (only while the file still has the
  // agent's lines there, crates/core/src/hunks.rs); Undo file and Keep work per file. J and K
  // (or Alt+F5 and Shift+Alt+F5) move between changes, Shift+U undoes the current one. Inline
  // or side by side is a setting (app.settings.diffLayout).
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick, untrack } from "svelte";
  import { ArrowCounterClockwise, CaretDown, CaretUp, Check, Columns, Rows } from "phosphor-svelte";
  import { api, errText } from "../api";
  import { highlightLines, type Span } from "../diffHighlight";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { imageBlob, mediaKind } from "../media";
  import { diffBlocks, isReviewed, pairRows, reviewItems, stepHunk, type DiffLine, type ReviewItem } from "../review";
  import { app, type Project } from "../stores/app.svelte";
  import { editor, type DiffTab } from "../stores/editor.svelte";
  import type { FileDiff } from "../types";
  import FileIcon from "./FileIcon.svelte";
  import ImageView from "./ImageView.svelte";

  let { project, tab }: { project: Project; tab: DiffTab } = $props();

  const session = $derived(app.sessionFor(tab.sessionKey));
  const items = $derived.by<ReviewItem[]>(() => {
    if (session) {
      const list = reviewItems(session.entries, tab.scope === "session" ? null : tab.turnId);
      if (list.length || tab.scope === "session") return list;
    }
    return tab.paths.map((path) => ({ turnId: tab.turnId, file: { path, status: "edit" as const, added: 0, removed: 0, binary: false } }));
  });
  const turnCount = $derived(session ? new Set(reviewItems(session.entries, null).map((x) => x.turnId)).size : 0);
  const split = $derived(app.settings.diffLayout === "split");

  const keyOf = (it: ReviewItem) => `${it.turnId}\u0000${it.file.path}`;

  type Loaded = FileDiff | { error: string };
  let diffs = $state<Record<string, Loaded>>({});
  let colors = $state.raw<Record<string, { before: Span[][] | null; after: Span[][] | null }>>({});
  let images = $state.raw<Record<string, { before: Blob | null; after: Blob | null }>>({});
  let expanded = $state<Record<string, number[]>>({});
  let busy = $state<string | null>(null);
  let root: HTMLDivElement | undefined = $state();

  const isDiff = (d: Loaded | undefined): d is FileDiff => !!d && !("error" in d);

  async function load(it: ReviewItem) {
    const k = keyOf(it);
    try {
      const d = await api.fileDiff(project.id, it.turnId, it.file.path);
      diffs[k] = d;
      if (!colors[k] && !d.binary) {
        const [before, after] = await Promise.all([highlightLines(d.path, d.before), highlightLines(d.path, d.after)]);
        colors = { ...colors, [k]: { before, after } };
      }
      if (d.binary && mediaKind(d.path) === "image" && !images[k]) loadImages(k, it);
    } catch (e) {
      diffs[k] = { error: errText(e) };
    }
  }

  async function loadImages(k: string, it: ReviewItem) {
    const side = (after: boolean) =>
      invoke<ArrayBuffer>("turn_file_bytes", { projectId: project.id, turnId: it.turnId, path: it.file.path, after }).then(
        (b) => imageBlob(it.file.path, b),
        () => null,
      );
    const [before, after] = await Promise.all([side(false), side(true)]);
    if (before || after) images = { ...images, [k]: { before, after } };
  }

  /** Loads every file, four at a time. */
  async function loadAll(list: ReviewItem[]) {
    const queue = list.filter((it) => !diffs[keyOf(it)]);
    const worker = async () => {
      for (let it = queue.shift(); it; it = queue.shift()) await load(it);
    };
    await Promise.all([worker(), worker(), worker(), worker()]);
  }

  let loadedFor = "";
  $effect(() => {
    const list = items;
    const sig = project.id + "\n" + list.map(keyOf).join("\n");
    if (sig === loadedFor) return;
    loadedFor = sig;
    untrack(() => loadAll(list));
  });

  // A file changed on disk: its hunk states may have changed.
  onMount(() => {
    const un = listen<{ projectId: string; paths: string[] }>("fs-changed", (e) => {
      if (e.payload.projectId !== project.id) return;
      const changed = new Set(e.payload.paths);
      for (const it of items) if (changed.has(it.file.path) && diffs[keyOf(it)]) load(it);
    });
    return () => {
      un.then((f) => f());
    };
  });

  // Every hunk of the review in order, for J and K.
  const allHunks = $derived(
    items.flatMap((it) => {
      const d = diffs[keyOf(it)];
      return isDiff(d) ? d.hunks.map((_, index) => ({ it, index })) : [];
    }),
  );
  const offsets = $derived.by(() => {
    const out: Record<string, number> = {};
    let n = 0;
    for (const it of items) {
      out[keyOf(it)] = n;
      const d = diffs[keyOf(it)];
      if (isDiff(d)) n += d.hunks.length;
    }
    return out;
  });
  let cur = $state(-1);

  async function go(step: 1 | -1) {
    cur = stepHunk(cur, allHunks.length, step);
    await tick();
    const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    root?.querySelector(`[data-hunk="${cur}"]`)?.scrollIntoView({ block: "center", behavior: calm ? "auto" : "smooth" });
  }

  // Opening the review at a file scrolls to it.
  $effect(() => {
    void tab.seq;
    const i = tab.index;
    untrack(async () => {
      cur = -1;
      await tick();
      root?.focus({ preventScroll: true });
      if (i > 0) root?.querySelector(`[data-file="${i}"]`)?.scrollIntoView({ block: "start" });
      else root?.scrollTo({ top: 0 });
    });
  });

  async function undoHunk(it: ReviewItem, index: number) {
    const k = keyOf(it);
    busy = `${k}#${index}`;
    try {
      diffs[k] = await api.undoHunk(project.id, it.turnId, it.file.path, index);
      app.flash(t("review.undoneChange"));
    } catch (e) {
      const m = errText(e);
      app.flash(m === "changedSince" ? t("review.changedSince") : m, "error");
      await load(it);
    }
    busy = null;
  }

  async function undoFile(it: ReviewItem) {
    busy = keyOf(it);
    await app.undoFile(session, project.id, it.turnId, it.file.path);
    await load(it);
    busy = null;
  }

  const entryOf = (it: ReviewItem) => (session ? app.turnEntry(session, it.turnId) : undefined);
  const kept = (it: ReviewItem) => {
    const e = entryOf(it);
    return !!e && isReviewed(e, it.file.path);
  };
  function toggleKeep(it: ReviewItem) {
    if (session) app.setReviewed(session, it.turnId, it.file.path, !kept(it));
  }
  function keepAll() {
    if (!session) return;
    for (const id of new Set(items.map((x) => x.turnId))) app.keepAll(session, id);
  }

  function setLayout(v: "inline" | "split") {
    app.settings.diffLayout = v;
    app.save();
  }

  function expand(k: string, at: number) {
    expanded[k] = [...(expanded[k] ?? []), at];
  }

  function onkeydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    const typing = !!el && (el.tagName === "TEXTAREA" || el.tagName === "INPUT" || el.isContentEditable);
    if (typing || e.ctrlKey || e.metaKey || app.overlay || app.undoTarget) return;
    const k = e.key;
    if (e.key === "F5" && e.altKey) go(e.shiftKey ? -1 : 1);
    else if (e.altKey) return;
    else if (!e.shiftKey && (k === "j" || k === "J")) go(1);
    else if (!e.shiftKey && (k === "k" || k === "K")) go(-1);
    else if (e.shiftKey && (k === "U" || k === "u")) {
      const h = allHunks[cur];
      const d = h && diffs[keyOf(h.it)];
      if (h && isDiff(d) && d.hunkUndo && d.hunks[h.index]?.state === "agent") undoHunk(h.it, h.index);
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }

  function word(status: string) {
    return status === "create" ? t("plan.change.create") : status === "delete" ? t("plan.change.delete") : status === "rename" ? t("plan.change.rename") : t("plan.change.edit");
  }

  /** The prompt a turn belongs to, for the headings of a session review. */
  function turnLabel(turnId: string, n: number): string {
    const p = session?.promptList.find((x) => x.turnId === turnId)?.prompt.trim().split("\n")[0];
    return p ? t("review.turnPrompt", { prompt: p.length > 90 ? p.slice(0, 87) + "..." : p }) : t("review.turnN", { n });
  }

  const spansOf = (k: string, line: DiffLine, side: "before" | "after"): Span[] | null => {
    const c = colors[k];
    const n = side === "before" ? line.old : line.new;
    const list = c && n ? (side === "before" ? c.before : c.after) : null;
    return list?.[n! - 1] ?? null;
  };
</script>

{#snippet text(k: string, line: DiffLine, side: "before" | "after")}
  {@const spans = spansOf(k, line, side)}
  {#if spans}{#each spans as s, i (i)}<span class={s.cls}>{s.text}</span>{/each}{:else}{line.text}{/if}
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="review" bind:this={root} tabindex="-1" role="region" {onkeydown} aria-label={t("review.label")}>
  <div class="bar">
    <span class="text-fg-muted">{t("review.files", { count: items.length })} . {t("review.changes", { count: allHunks.length })}</span>
    <span class="flex-1"></span>
    {#if turnCount > 1 && session}
      <div class="seg" role="group" aria-label={t("review.scope")}>
        <button class:on={tab.scope !== "session"} aria-pressed={tab.scope !== "session"} onclick={() => session && app.reviewTurn(session, tab.turnId)}>{t("review.thisTurn")}</button>
        <button class:on={tab.scope === "session"} aria-pressed={tab.scope === "session"} onclick={() => app.reviewSession(session)}>{t("review.session")}</button>
      </div>
    {/if}
    <div class="seg" role="group" aria-label={t("review.layout")}>
      <button class:on={!split} aria-pressed={!split} title={t("review.inline")} onclick={() => setLayout("inline")}><Rows size={13} />{t("review.inline")}</button>
      <button class:on={split} aria-pressed={split} title={t("review.split")} onclick={() => setLayout("split")}><Columns size={13} />{t("review.split")}</button>
    </div>
    <button class="icon-btn" onclick={() => go(-1)} disabled={!allHunks.length} title="{t('keys.prevChange')} (K)" aria-label={t("keys.prevChange")}><CaretUp size={14} /></button>
    <button class="icon-btn" onclick={() => go(1)} disabled={!allHunks.length} title="{t('keys.nextChange')} (J)" aria-label={t("keys.nextChange")}><CaretDown size={14} /></button>
    {#if session}<button class="btn h-6" onclick={keepAll}><Check size={13} />{t("review.keepAll")}</button>{/if}
  </div>

  {#each items as it, fi (keyOf(it))}
    {@const k = keyOf(it)}
    {@const d = diffs[k]}
    {#if tab.scope === "session" && (fi === 0 || items[fi - 1].turnId !== it.turnId)}
      <p class="turn">{turnLabel(it.turnId, new Set(items.slice(0, fi + 1).map((x) => x.turnId)).size)}</p>
    {/if}
    <section class="file" data-file={fi} aria-label={it.file.path}>
      <header class="fhead">
        <span class="w-12 shrink-0 text-fg-muted">{word(it.file.status)}</span>
        <FileIcon path={it.file.path} />
        <button class="path truncate" title={t("review.openFile")} onclick={() => editor.open(project.id, it.file.path, { pin: true })}>
          {#if it.file.oldPath}<span class="text-fg-muted">{it.file.oldPath} &rarr; </span>{/if}{it.file.path}
        </button>
        {#if it.file.added}<span class="text-add">+{it.file.added}</span>{/if}
        {#if it.file.removed}<span class="text-del">-{it.file.removed}</span>{/if}
        {#if isDiff(d) && d.current === "changed"}<span class="note text-warn">{t("review.fileChangedNow")}</span>{/if}
        {#if isDiff(d) && d.current === "missing" && it.file.status !== "delete"}<span class="note text-warn">{t("review.fileMissing")}</span>{/if}
        <span class="flex-1"></span>
        <button class="btn h-6" onclick={() => undoFile(it)} disabled={busy === k}><ArrowCounterClockwise size={13} />{t("review.undoFile")}</button>
        {#if session}
          <button class="btn h-6" class:kept={kept(it)} aria-pressed={kept(it)} onclick={() => toggleKeep(it)}>
            <Check size={13} />{kept(it) ? t("review.kept") : t("review.keep")}
          </button>
        {/if}
      </header>

      {#if !d}
        <p class="msg text-fg-muted">{t("common.loading")}</p>
      {:else if "error" in d}
        <p class="msg text-danger selectable">{d.error}</p>
      {:else if d.binary}
        {@const img = images[k]}
        {#if img}
          <div class="sides">
            {#each [{ label: t("viewer.before"), blob: img.before }, { label: t("viewer.after"), blob: img.after }] as side (side.label)}
              <div class="side">
                <p class="side-label">{side.label}</p>
                {#if side.blob}<ImageView blob={side.blob} label="{side.label}: {it.file.path}" compact />{:else}<p class="msg text-fg-muted">{t("viewer.noVersion")}</p>{/if}
              </div>
            {/each}
          </div>
        {:else}
          <p class="msg text-fg-muted">{t("review.binary")}</p>
        {/if}
      {:else if !d.hunks.length}
        <p class="msg text-fg-muted">{t("review.noLines")}</p>
      {:else}
        <div class="code selectable" class:split>
          {#each diffBlocks(d.before, d.after, d.hunks, new Set(expanded[k] ?? [])) as b, bi (bi)}
            {#if b.kind === "gap"}
              <button class="gap" onclick={() => expand(k, b.at)}>{t("review.unchanged", { count: b.count })}</button>
            {:else if b.kind === "context"}
              {#each b.lines as l, li (li)}
                {#if split}
                  <div class="row"><span class="no">{l.old}</span><span class="tx">{@render text(k, l, "before")}</span><span class="no">{l.new}</span><span class="tx">{@render text(k, l, "after")}</span></div>
                {:else}
                  <div class="row"><span class="no">{l.old}</span><span class="no">{l.new}</span><span class="sg"></span><span class="tx">{@render text(k, l, "after")}</span></div>
                {/if}
              {/each}
            {:else}
              {@const gi = offsets[k] + b.index}
              <div class="hbar" class:current={cur === gi} data-hunk={gi}>
                <span class="text-fg-muted">{t("review.change", { index: b.index + 1, count: d.hunks.length })}</span>
                <span class="flex-1"></span>
                {#if b.state === "undone"}
                  <span class="text-fg-muted">{t("review.undone")}</span>
                {:else if b.state === "changed"}
                  <span class="text-warn">{t("review.changedSinceShort")}</span>
                {:else if d.hunkUndo}
                  <button class="btn h-6" disabled={busy === `${k}#${b.index}`} onclick={() => undoHunk(it, b.index)} title={keyLabel("Shift+U")}>
                    <ArrowCounterClockwise size={13} />{t("review.undoChange")}
                  </button>
                {/if}
              </div>
              {#if split}
                {#each pairRows(b.del, b.add) as r, ri (ri)}
                  <div class="row" class:faded={b.state === "undone"}>
                    <span class="no" class:del={!!r.left}>{r.left?.old ?? ""}</span><span class="tx" class:del={!!r.left} class:empty={!r.left}>{#if r.left}{@render text(k, r.left, "before")}{/if}</span>
                    <span class="no" class:add={!!r.right}>{r.right?.new ?? ""}</span><span class="tx" class:add={!!r.right} class:empty={!r.right}>{#if r.right}{@render text(k, r.right, "after")}{/if}</span>
                  </div>
                {/each}
              {:else}
                {#each b.del as l, li ("d" + li)}
                  <div class="row del" class:faded={b.state === "undone"}><span class="no">{l.old}</span><span class="no"></span><span class="sg">-</span><span class="tx">{@render text(k, l, "before")}</span></div>
                {/each}
                {#each b.add as l, li ("a" + li)}
                  <div class="row add" class:faded={b.state === "undone"}><span class="no"></span><span class="no">{l.new}</span><span class="sg">+</span><span class="tx">{@render text(k, l, "after")}</span></div>
                {/each}
              {/if}
            {/if}
          {/each}
        </div>
      {/if}
    </section>
  {:else}
    <p class="msg text-fg-muted">{t("review.nothing")}</p>
  {/each}
</div>

<style>
  .review {
    flex: 1;
    min-height: 0;
    overflow: auto;
    outline: none;
  }
  .bar {
    position: sticky;
    top: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    min-width: 0;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    overflow: hidden;
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 7px;
    border: 0;
    background: transparent;
    color: var(--fg-muted);
    font-size: 12px;
  }
  .seg button + button {
    border-inline-start: 1px solid var(--border-strong);
  }
  .seg button.on {
    background: var(--bg-active);
    color: var(--fg);
  }
  .turn {
    padding: 14px 12px 4px;
    font-weight: 600;
    color: var(--fg-muted);
  }
  .file {
    border-bottom: 1px solid var(--border);
  }
  .fhead {
    position: sticky;
    top: 30px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px 0 12px;
    background: var(--bg-subtle);
    border-bottom: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 12.5px;
    min-width: 0;
  }
  .fhead .btn,
  .fhead .note {
    font-family: var(--ui-font);
    font-size: 12px;
  }
  .note {
    white-space: nowrap;
  }
  .path {
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 0;
    text-align: start;
    color: var(--fg);
    font: inherit;
  }
  .path:hover {
    text-decoration: underline;
  }
  .btn.kept {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
  }
  .msg {
    padding: 10px 14px;
  }
  .code {
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.55;
    padding-bottom: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 4.5ch 4.5ch 2ch minmax(0, 1fr);
  }
  .split .row {
    grid-template-columns: 4.5ch minmax(0, 1fr) 4.5ch minmax(0, 1fr);
  }
  .no {
    padding-inline-end: 6px;
    text-align: end;
    color: var(--fg-faint);
    user-select: none;
  }
  .sg {
    color: var(--fg-faint);
    user-select: none;
  }
  .tx {
    white-space: pre-wrap;
    word-break: break-word;
    min-width: 0;
    padding-inline-end: 8px;
  }
  .split .tx + .no {
    border-inline-start: 1px solid var(--border);
  }
  .row.del,
  .split .del {
    background: var(--del-bg);
  }
  .row.add,
  .split .add {
    background: var(--add-bg);
  }
  .row.del .sg {
    color: var(--del);
  }
  .row.add .sg {
    color: var(--add);
  }
  .split .empty {
    background: color-mix(in srgb, var(--fg) 3%, transparent);
  }
  .faded {
    opacity: 0.5;
  }
  .hbar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    margin-top: 4px;
    padding: 0 8px 0 12px;
    font-family: var(--ui-font);
    font-size: 12px;
    border-top: 1px solid var(--border);
    scroll-margin-top: 70px;
  }
  .hbar.current {
    box-shadow: inset 3px 0 0 var(--project);
    background: color-mix(in srgb, var(--project) 8%, transparent);
  }
  .gap {
    display: block;
    width: 100%;
    height: 24px;
    border: 0;
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-family: var(--ui-font);
    font-size: 12px;
    text-align: start;
    padding-inline-start: 12px;
  }
  .gap:hover {
    color: var(--fg);
    background: var(--bg-hover);
  }
  .sides {
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
  /* Syntax colors (lib/diffHighlight.ts, Lezer's class highlighter). */
  .code :global(.tok-keyword),
  .code :global(.tok-operatorKeyword) {
    color: var(--syn-keyword);
  }
  .code :global(.tok-string),
  .code :global(.tok-string2) {
    color: var(--syn-string);
  }
  .code :global(.tok-number),
  .code :global(.tok-bool),
  .code :global(.tok-atom) {
    color: var(--syn-number);
  }
  .code :global(.tok-comment) {
    color: var(--syn-comment);
    font-style: italic;
  }
  .code :global(.tok-meta) {
    color: var(--syn-comment);
  }
  .code :global(.tok-typeName),
  .code :global(.tok-className),
  .code :global(.tok-namespace) {
    color: var(--syn-type);
  }
  .code :global(.tok-propertyName) {
    color: var(--syn-prop);
  }
  .code :global(.tok-definition),
  .code :global(.tok-macroName) {
    color: var(--syn-def);
  }
  .code :global(.tok-heading) {
    color: var(--syn-tag);
    font-weight: 600;
  }
  .code :global(.tok-invalid) {
    color: var(--danger);
  }
  @media (prefers-reduced-motion: reduce) {
    .review {
      scroll-behavior: auto;
    }
  }
</style>
