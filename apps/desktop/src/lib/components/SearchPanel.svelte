<script lang="ts">
  // The Search panel (dock id "search"): search in all project files with match case, whole
  // word, regular expression, and include and exclude globs. Results are grouped by file with
  // a preview of each line; a click opens the file at the match. Replace all shows the files it
  // changes first, and Undo puts them back (a snapshot is taken before it writes).
  import { CaretDown, CaretRight, X } from "phosphor-svelte";
  import { api, errText, type FileMatches, type SearchHit } from "../api";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";
  import { editor } from "../stores/editor.svelte";
  import { search as s } from "../stores/search.svelte";
  import DockGrip from "./DockGrip.svelte";
  import FileIcon from "./FileIcon.svelte";

  let { project }: { project: Project } = $props();

  let list: HTMLDivElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | null = null;
  let run = 0;
  /** The Replace all preview, while the user confirms. */
  let confirm = $state<{ files: FileMatches[]; unsaved: string[] } | null>(null);
  let replacing = $state(false);
  let confirmBtn: HTMLButtonElement | undefined = $state();

  const query = () => ({
    query: s.query,
    caseSensitive: s.caseSensitive,
    wholeWord: s.wholeWord,
    regex: s.regex,
    include: s.include,
    exclude: s.exclude,
  });

  $effect(() => {
    const q = query();
    const pid = project.id;
    void s.rerun;
    if (timer) clearTimeout(timer);
    if (!q.query) {
      s.hits = [];
      s.error = null;
      s.truncated = false;
      s.busy = false;
      return;
    }
    timer = setTimeout(async () => {
      const mine = ++run;
      s.busy = true;
      try {
        const res = await api.fsSearch(pid, q);
        if (mine !== run) return;
        s.hits = res.hits;
        s.truncated = res.truncated;
        s.projectId = pid;
        s.error = null;
      } catch (e) {
        if (mine === run) {
          s.error = errText(e);
          s.hits = [];
        }
      } finally {
        if (mine === run) s.busy = false;
      }
    }, 200);
  });

  /** Hits grouped by file, in result order. */
  const groups = $derived.by(() => {
    const out: { path: string; hits: SearchHit[] }[] = [];
    for (const hit of s.hits) {
      let g = out[out.length - 1];
      if (!g || g.path !== hit.path) out.push((g = { path: hit.path, hits: [] }));
      g.hits.push(hit);
    }
    return out;
  });

  function open(h: SearchHit, pin = false) {
    const len = h.ranges[0] ? h.ranges[0][1] - h.ranges[0][0] : 0;
    editor.open(project.id, h.path, { pin, line: h.line, col: h.col, len });
  }

  /** Splits a preview line at the matches. */
  function parts(h: SearchHit): { s: string; m: boolean }[] {
    const out: { s: string; m: boolean }[] = [];
    let last = 0;
    for (const [a, b] of h.ranges) {
      if (a < last) continue;
      out.push({ s: h.text.slice(last, a), m: false }, { s: h.text.slice(a, b), m: true });
      last = b;
    }
    out.push({ s: h.text.slice(last), m: false });
    return out;
  }

  /** Arrow keys move between results; from the search field, Down goes to the first one. */
  function onlistkey(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const items = [...(list?.querySelectorAll<HTMLElement>("[data-item]") ?? [])];
    const i = items.indexOf(document.activeElement as HTMLElement);
    const next = e.key === "ArrowDown" ? items[Math.min(items.length - 1, i + 1)] : items[i - 1];
    if (next) {
      e.preventDefault();
      next.focus();
    }
  }

  function onfieldkey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      const first = list?.querySelector<HTMLElement>("[data-item]");
      if (first) {
        e.preventDefault();
        first.focus();
      }
    } else if (e.key === "Enter" && s.hits[0]) {
      e.preventDefault();
      open(s.hits[0]);
    } else if (e.altKey && !e.ctrlKey && !e.metaKey && ["c", "w", "r"].includes(e.key.toLowerCase())) {
      e.preventDefault();
      const k = e.key.toLowerCase();
      if (k === "c") s.caseSensitive = !s.caseSensitive;
      else if (k === "w") s.wholeWord = !s.wholeWord;
      else s.regex = !s.regex;
    }
  }

  async function askReplace() {
    if (!s.query || replacing) return;
    try {
      const files = await api.fsReplacePreview(project.id, query());
      if (!files.length) {
        app.flash(t("search.empty"));
        return;
      }
      const w = editor.get(project.id);
      const unsaved = files.map((f) => f.path).filter((p) => w?.docs[p]?.dirty);
      confirm = { files, unsaved };
      queueMicrotask(() => confirmBtn?.focus());
    } catch (e) {
      app.flash(errText(e), "error");
    }
  }

  async function doReplace() {
    if (!confirm) return;
    const paths = confirm.files.map((f) => f.path);
    confirm = null;
    replacing = true;
    try {
      const out = await api.fsReplace(project.id, query(), s.replace, paths);
      const matches = out.files.reduce((n, f) => n + f.count, 0);
      s.lastReplace = out.id ? { projectId: project.id, id: out.id, matches, files: out.files.length } : null;
      if (out.failed.length) app.flash(t("search.replaceFailed", { count: out.failed.length, error: out.failed[0][1] }), "error");
      else if (!out.id) app.flash(t("search.replacedNoUndo", { count: matches, files: out.files.length }));
      s.rerun++;
    } catch (e) {
      app.flash(errText(e), "error");
    } finally {
      replacing = false;
    }
  }

  async function undoReplace() {
    const last = s.lastReplace;
    if (!last) return;
    s.lastReplace = null;
    try {
      const out = await api.fsReplaceUndo(last.projectId, last.id);
      if (out.failed.length) app.flash(out.failed[0][1], "error");
      else if (out.skipped.length) app.flash(t("search.undoneSome", { count: out.restored.length, skipped: out.skipped.length }));
      else app.flash(t("search.undone", { count: out.restored.length }));
      s.rerun++;
    } catch (e) {
      app.flash(errText(e), "error");
    }
  }

  function onconfirmkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      confirm = null;
    }
  }

  const fileCount = $derived(groups.length);
  const total = $derived(confirm ? confirm.files.reduce((n, f) => n + f.count, 0) : 0);
  const literalReplace = $derived(s.showReplace && !s.regex && s.replace !== "");
</script>

<section class="search" aria-label={t("search.title")}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="head" onpointerdown={(e) => dock.fromHeader("search", e)}>
    <span class="title">{t("search.title")}</span>
    <span class="flex-1"></span>
    <DockGrip panel="search" />
    <button class="icon-btn" title={t("common.close")} aria-label={t("dock.close", { panel: t("dock.search") })} onclick={() => dock.close("search")}>
      <X size={13} />
    </button>
  </div>

  <div class="form">
    <div class="line">
      <button
        class="icon-btn fold"
        aria-expanded={s.showReplace}
        aria-label={t("search.toggleReplace")}
        title={t("search.toggleReplace")}
        onclick={() => (s.showReplace = !s.showReplace)}
      >
        {#if s.showReplace}<CaretDown size={12} />{:else}<CaretRight size={12} />{/if}
      </button>
      <div class="field">
        <input data-dock-focus bind:value={s.query} placeholder={t("search.placeholder")} aria-label={t("search.placeholder")} spellcheck="false" onkeydown={onfieldkey} class:bad={!!s.error} />
        <button class="opt" aria-pressed={s.caseSensitive} title="{t('search.case')} (Alt+C)" aria-label={t("search.case")} onclick={() => (s.caseSensitive = !s.caseSensitive)}>Aa</button>
        <button class="opt word" aria-pressed={s.wholeWord} title="{t('search.word')} (Alt+W)" aria-label={t("search.word")} onclick={() => (s.wholeWord = !s.wholeWord)}>ab</button>
        <button class="opt" aria-pressed={s.regex} title="{t('search.regex')} (Alt+R)" aria-label={t("search.regex")} onclick={() => (s.regex = !s.regex)}>.*</button>
      </div>
    </div>
    {#if s.showReplace}
      <div class="line">
        <span class="fold-space"></span>
        <div class="field">
          <input bind:value={s.replace} placeholder={t("search.replacePlaceholder")} aria-label={t("search.replacePlaceholder")} spellcheck="false" />
          <button class="btn h-6" onclick={askReplace} disabled={!s.hits.length || replacing} title={t("search.replaceAllHint")}>{t("search.replaceAll")}</button>
        </div>
      </div>
    {/if}
    <div class="line">
      <span class="fold-space"></span>
      <button class="link" aria-expanded={s.showFilters} onclick={() => (s.showFilters = !s.showFilters)}>
        {s.showFilters ? t("search.hideFilters") : t("search.showFilters")}
      </button>
    </div>
    {#if s.showFilters}
      <label class="glob">
        <span>{t("search.include")}</span>
        <input bind:value={s.include} placeholder={t("search.globExample")} spellcheck="false" />
      </label>
      <label class="glob">
        <span>{t("search.exclude")}</span>
        <input bind:value={s.exclude} placeholder={t("search.excludeExample")} spellcheck="false" />
      </label>
    {/if}
  </div>

  {#if s.lastReplace && s.lastReplace.projectId === project.id}
    <div class="done" role="status">
      <span class="flex-1">{t("search.replaced", { count: s.lastReplace.matches, files: s.lastReplace.files })}</span>
      <button class="btn h-6" onclick={undoReplace}>{t("search.undo")}</button>
      <button class="icon-btn" aria-label={t("common.close")} title={t("common.close")} onclick={() => (s.lastReplace = null)}><X size={12} /></button>
    </div>
  {/if}

  <p class="summary" aria-live="polite">
    {#if s.error}
      <span class="text-danger selectable">{s.error}</span>
    {:else if s.query && s.busy && !s.hits.length}
      {t("search.searching")}
    {:else if s.query && !s.hits.length}
      {t("search.empty")}
    {:else if s.hits.length}
      {s.truncated ? t("search.truncated", { count: s.hits.length }) : t("search.countFiles", { count: s.hits.length, files: fileCount })}
    {/if}
  </p>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="list" bind:this={list} onkeydown={onlistkey}>
    {#each groups as g (g.path)}
      {@const folded = !!s.folded[g.path]}
      <button class="file" data-item aria-expanded={!folded} onclick={() => (s.folded[g.path] = !folded)} title={g.path}>
        {#if folded}<CaretRight size={10} class="shrink-0" />{:else}<CaretDown size={10} class="shrink-0" />{/if}
        <FileIcon path={g.path} size={14} />
        <span class="name truncate">{g.path.split("/").pop()}</span>
        <span class="dir truncate">{g.path.includes("/") ? g.path.slice(0, g.path.lastIndexOf("/")) : ""}</span>
        <span class="n">{g.hits.length}</span>
      </button>
      {#if !folded}
        {#each g.hits as hit (hit.line)}
          <button class="hit" data-item title="{g.path}:{hit.line}" onclick={() => open(hit)} ondblclick={() => open(hit, true)}>
            <span class="ln">{hit.line}</span>
            <span class="txt truncate"
              >{#if hit.cut}...{/if}{#each parts(hit) as p, k (k)}{#if p.m && literalReplace}<del>{p.s}</del><ins>{s.replace}</ins>{:else if p.m}<mark>{p.s}</mark>{:else}{p.s}{/if}{/each}</span
            >
          </button>
        {/each}
      {/if}
    {/each}
  </div>
</section>

{#if confirm}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => (confirm = null)} onkeydown={onconfirmkey}>
    <div class="dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="replace-title" onclick={(e) => e.stopPropagation()}>
      <h2 id="replace-title" class="mb-2 text-[15px] font-semibold">{t("search.confirmTitle")}</h2>
      <p class="mb-2">{t("search.confirmBody", { count: total, files: confirm.files.length, text: s.replace })}</p>
      <ul class="files">
        {#each confirm.files as f (f.path)}
          <li class="flex gap-2"><span class="mono truncate" title={f.path}>{f.path}</span><span class="ms-auto shrink-0 text-fg-muted">{f.count}</span></li>
        {/each}
      </ul>
      {#if confirm.unsaved.length}
        <p class="mt-2 text-warn">{t("search.confirmUnsaved", { files: confirm.unsaved.join(", ") })}</p>
      {/if}
      <p class="mt-2 text-fg-muted">{t("search.confirmUndo")}</p>
      <div class="mt-3 flex justify-end gap-2">
        <button class="btn" onclick={() => (confirm = null)}>{t("common.cancel")}</button>
        <button class="btn btn-primary" bind:this={confirmBtn} onclick={doReplace}>{t("search.replaceAll")}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .search {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-subtle);
    min-height: 0;
  }
  .head {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 8px 4px 4px;
    flex-shrink: 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .fold,
  .fold-space {
    width: 20px;
    flex-shrink: 0;
  }
  .field {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 2px;
    padding-inline-end: 3px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--bg);
  }
  .field:focus-within {
    border-color: var(--fg-faint);
  }
  .field input {
    flex: 1;
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    border: 0;
    background: transparent;
    color: var(--fg);
    outline: none;
    user-select: text;
  }
  .field:has(input.bad) {
    border-color: var(--danger);
  }
  .opt {
    display: inline-grid;
    place-items: center;
    min-width: 22px;
    height: 20px;
    padding: 0 3px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--fg-muted);
  }
  .opt.word {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .opt:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .opt[aria-pressed="true"] {
    background: var(--bg-active);
    border-color: var(--fg-faint);
    color: var(--fg);
  }
  .link {
    border: 0;
    padding: 0 4px;
    background: transparent;
    color: var(--fg-muted);
    font-size: 12px;
  }
  .link:hover {
    color: var(--fg);
    text-decoration: underline;
  }
  .glob {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-inline-start: 22px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .glob input {
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
    font-size: var(--ui-size);
    outline: none;
    user-select: text;
  }
  .glob input:focus {
    border-color: var(--fg-faint);
  }
  .done {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 4px 8px;
    padding: 4px 4px 4px 10px;
    border-radius: 6px;
    background: var(--accent-soft);
  }
  .summary {
    padding: 4px 12px 4px 26px;
    min-height: 22px;
    color: var(--fg-muted);
    font-size: 12px;
    flex-shrink: 0;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 12px;
  }
  .file,
  .hit {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 24px;
    border: 0;
    background: transparent;
    text-align: start;
    min-width: 0;
    white-space: nowrap;
  }
  .file {
    padding: 0 10px 0 8px;
  }
  .hit {
    padding: 0 10px 0 28px;
    color: var(--fg-muted);
  }
  .file:hover,
  .hit:hover {
    background: var(--bg-hover);
  }
  .file:focus-visible,
  .hit:focus-visible {
    outline-offset: -2px;
  }
  .name {
    font-weight: 500;
    min-width: 0;
    flex-shrink: 0;
    max-width: 60%;
  }
  .dir {
    min-width: 0;
    font-size: 11.5px;
    color: var(--fg-faint);
  }
  .n {
    margin-inline-start: auto;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--bg-active);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .ln {
    min-width: 28px;
    flex-shrink: 0;
    text-align: end;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
    font-size: 11.5px;
  }
  .txt {
    min-width: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg);
  }
  mark {
    background: var(--warn-soft);
    color: inherit;
    outline: 1px solid color-mix(in srgb, var(--warn-line) 50%, transparent);
    border-radius: 2px;
  }
  del {
    background: var(--del-bg);
    color: var(--del);
  }
  ins {
    background: var(--add-bg);
    color: var(--add);
    text-decoration: none;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 60;
    display: grid;
    place-items: center;
  }
  .dialog {
    width: 520px;
    max-width: calc(100vw - 32px);
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 18px 20px 16px;
    outline: none;
  }
  .files {
    max-height: 220px;
    overflow-y: auto;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
</style>
