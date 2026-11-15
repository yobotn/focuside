<script lang="ts">
  // Quick open (Mod+P): fuzzy search over the project's files (the ignore rules apply), recent
  // files first. Enter opens the file as a kept tab. "name:12" opens it at line 12, ":12" goes
  // to line 12 in the open file.
  import { allFiles } from "../files.svelte";
  import { score } from "../fuzzy";
  import { t } from "../i18n";
  import type { Project } from "../stores/app.svelte";
  import { editor } from "../stores/editor.svelte";
  import FileIcon from "./FileIcon.svelte";

  let { project, onclose }: { project: Project; onclose: () => void } = $props();

  let query = $state("");
  let index = $state(0);
  let files = $state<string[]>([]);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLUListElement | undefined = $state();

  $effect(() => {
    allFiles(project.id, true).then((f) => (files = f));
    queueMicrotask(() => input?.focus());
  });

  const w = $derived(editor.get(project.id));
  const recent = $derived.by(() => {
    const cur = w?.active;
    const seen = w?.recent ?? [];
    // The file shown now goes last: the one before it is the usual pick.
    return [...seen.filter((p) => p !== cur), ...(cur && seen.includes(cur) ? [cur] : [])];
  });

  /** "name:12" or ":12". */
  const parsed = $derived.by(() => {
    const m = /^(.*?):(\d+)?$/.exec(query.trim());
    if (!m) return { name: query.trim(), line: 0 };
    return { name: m[1].trim(), line: m[2] ? Number(m[2]) : 0 };
  });

  interface Row {
    path: string;
    recent: boolean;
  }

  const rows = $derived.by<Row[]>(() => {
    const name = parsed.name.replace(/\\/g, "/");
    if (!name) {
      if (parsed.line) return [];
      const rec = recent.map((path) => ({ path, recent: true }));
      const rest = files.filter((f) => !recent.includes(f)).slice(0, 100).map((path) => ({ path, recent: false }));
      return [...rec, ...rest];
    }
    const scored: [number, Row][] = [];
    const rank = new Map(recent.map((p, i) => [p, i]));
    for (const f of new Set([...recent, ...files])) {
      let sc = score(name, f);
      if (sc <= 0) continue;
      const r = rank.get(f);
      if (r !== undefined) sc += 20 - Math.min(19, r);
      scored.push([sc, { path: f, recent: r !== undefined }]);
    }
    scored.sort((a, b) => b[0] - a[0]);
    return scored.slice(0, 100).map((x) => x[1]);
  });

  /** Only ":12": a line in the file that is open now. */
  const lineOnly = $derived(!parsed.name && parsed.line > 0 && w?.current?.kind === "file" ? w.current.path : null);

  $effect(() => {
    void query;
    index = 0;
  });
  $effect(() => {
    list?.children[index]?.scrollIntoView({ block: "nearest" });
  });

  function pick(i: number) {
    if (lineOnly) {
      onclose();
      editor.open(project.id, lineOnly, { line: parsed.line });
      return;
    }
    const r = rows[i];
    if (!r) return;
    onclose();
    editor.open(project.id, r.path, { pin: true, line: parsed.line || undefined });
  }

  function onkeydown(e: KeyboardEvent) {
    const n = lineOnly ? 1 : rows.length;
    if (e.key === "ArrowDown") index = Math.min(n - 1, index + 1);
    else if (e.key === "ArrowUp") index = Math.max(0, index - 1);
    else if (e.key === "PageDown") index = Math.min(n - 1, index + 10);
    else if (e.key === "PageUp") index = Math.max(0, index - 10);
    else if (e.key === "Enter") pick(index);
    else if (e.key === "Escape") onclose();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  const dirOf = (p: string) => (p.includes("/") ? p.slice(0, p.lastIndexOf("/")) : "");
  const nameOf = (p: string) => p.split("/").pop() ?? p;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div class="panel" onclick={(e) => e.stopPropagation()} {onkeydown} role="dialog" tabindex="-1" aria-label={t("quick.title")}>
    <input
      bind:this={input}
      bind:value={query}
      placeholder={t("quick.placeholder")}
      class="search"
      spellcheck="false"
      role="combobox"
      aria-expanded="true"
      aria-controls="quick-list"
      aria-activedescendant={rows.length || lineOnly ? `quick-${index}` : undefined}
      aria-label={t("quick.title")}
    />
    {#if lineOnly}
      <ul class="list" id="quick-list" role="listbox" bind:this={list}>
        <li id="quick-0" role="option" aria-selected="true">
          <button class="item active" onclick={() => pick(0)}>
            <FileIcon path={lineOnly} size={14} />
            <span class="truncate">{t("quick.gotoLine", { line: parsed.line, name: nameOf(lineOnly) })}</span>
          </button>
        </li>
      </ul>
    {:else if !rows.length}
      <p class="px-4 py-3 text-fg-muted">{parsed.line && !parsed.name ? t("quick.noFile") : t("palette.empty")}</p>
    {:else}
      <ul class="list" id="quick-list" role="listbox" bind:this={list}>
        {#each rows as r, i (r.path)}
          <li id="quick-{i}" role="option" aria-selected={i === index}>
            <button class="item" class:active={i === index} tabindex="-1" onclick={() => pick(i)} onmousemove={() => (index = i)} title={r.path}>
              <FileIcon path={r.path} size={14} />
              <span class="name">{nameOf(r.path)}</span>
              <span class="dir truncate">{dirOf(r.path)}</span>
              {#if r.recent && (i === 0 || !rows[i - 1].recent)}<span class="tag">{t("quick.recent")}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="foot">{t("quick.hint")}</p>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 50;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .panel {
    width: 640px;
    max-width: calc(100vw - 32px);
    max-height: 64vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    overflow: hidden;
    outline: none;
  }
  .search {
    height: 44px;
    padding: 0 14px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--fg);
    font-size: 14px;
    outline: none;
    user-select: text;
  }
  .list {
    overflow-y: auto;
    padding: 4px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    text-align: start;
    min-width: 0;
    white-space: nowrap;
  }
  .item.active {
    background: var(--bg-hover);
  }
  .name {
    flex-shrink: 0;
  }
  .dir {
    min-width: 0;
    color: var(--fg-muted);
    font-size: 12px;
  }
  .tag {
    margin-inline-start: auto;
    flex-shrink: 0;
    padding-inline-start: 12px;
    color: var(--fg-faint);
    font-size: 11.5px;
  }
  .foot {
    padding: 6px 14px;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 12px;
  }
</style>
