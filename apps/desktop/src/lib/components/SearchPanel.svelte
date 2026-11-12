<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { errText } from "../api";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import FileIcon from "./FileIcon.svelte";

  interface Hit {
    path: string;
    line: number;
    text: string;
  }

  let { project, onclose }: { project: Project; onclose: () => void } = $props();

  let query = $state("");
  let caseSensitive = $state(false);
  let regex = $state(false);
  let hits = $state<Hit[]>([]);
  let truncated = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let index = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLDivElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | null = null;
  let run = 0;

  $effect(() => {
    queueMicrotask(() => input?.focus());
  });

  $effect(() => {
    const q = query;
    const c = caseSensitive;
    const r = regex;
    if (timer) clearTimeout(timer);
    if (!q) {
      hits = [];
      error = null;
      return;
    }
    timer = setTimeout(async () => {
      const mine = ++run;
      busy = true;
      try {
        const res = await invoke<{ hits: Hit[]; truncated: boolean }>("fs_search", {
          projectId: project.id,
          query: q,
          caseSensitive: c,
          regex: r,
        });
        if (mine !== run) return;
        hits = res.hits;
        truncated = res.truncated;
        error = null;
        index = 0;
      } catch (e) {
        if (mine === run) error = errText(e);
      } finally {
        if (mine === run) busy = false;
      }
    }, 180);
  });

  /** Hits grouped by file, in result order. */
  const groups = $derived.by(() => {
    const out: { path: string; hits: { hit: Hit; i: number }[] }[] = [];
    hits.forEach((hit, i) => {
      let g = out[out.length - 1];
      if (!g || g.path !== hit.path) out.push((g = { path: hit.path, hits: [] }));
      g.hits.push({ hit, i });
    });
    return out;
  });

  $effect(() => {
    list?.querySelector(`[data-i="${index}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function open(i: number) {
    const h = hits[i];
    if (!h) return;
    app.viewer = { kind: "file", projectId: project.id, path: h.path };
    onclose();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") index = Math.min(hits.length - 1, index + 1);
    else if (e.key === "ArrowUp") index = Math.max(0, index - 1);
    else if (e.key === "Enter") open(index);
    else if (e.key === "Escape") onclose();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  /** Splits a line around the matches of the query, for highlighting. */
  function parts(text: string): { s: string; m: boolean }[] {
    try {
      const re = new RegExp(regex ? query : query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), caseSensitive ? "g" : "gi");
      const out: { s: string; m: boolean }[] = [];
      let last = 0;
      for (const m of text.matchAll(re)) {
        if (m[0] === "") break;
        out.push({ s: text.slice(last, m.index), m: false }, { s: m[0], m: true });
        last = (m.index ?? 0) + m[0].length;
      }
      out.push({ s: text.slice(last), m: false });
      return out;
    } catch {
      return [{ s: text, m: false }];
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div class="panel" onclick={(e) => e.stopPropagation()} {onkeydown} role="dialog" tabindex="-1" aria-label={t("search.title")}>
    <div class="top">
      <input bind:this={input} bind:value={query} placeholder={t("search.placeholder")} spellcheck="false" />
      <label class="opt" title={t("search.case")}><input type="checkbox" bind:checked={caseSensitive} />Aa</label>
      <label class="opt" title={t("search.regex")}><input type="checkbox" bind:checked={regex} />.*</label>
    </div>
    <div class="list" bind:this={list}>
      {#if error}
        <p class="px-4 py-3 text-danger">{error}</p>
      {:else if query && !busy && !hits.length}
        <p class="px-4 py-3 text-fg-muted">{t("search.empty")}</p>
      {/if}
      {#each groups as g (g.path)}
        <div class="file"><FileIcon path={g.path} size={14} /><span class="truncate">{g.path}</span></div>
        {#each g.hits as { hit, i } (i)}
          <button class="hit" class:active={i === index} data-i={i} onclick={() => open(i)} onmousemove={() => (index = i)}>
            <span class="ln">{hit.line}</span>
            <span class="txt truncate">{#each parts(hit.text.trimStart()) as p, k (k)}{#if p.m}<mark>{p.s}</mark>{:else}{p.s}{/if}{/each}</span>
          </button>
        {/each}
      {/each}
    </div>
    {#if hits.length}
      <p class="foot">
        {truncated ? t("search.truncated", { count: hits.length }) : t("search.count", { count: hits.length })}
      </p>
    {/if}
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
    padding-top: 10vh;
  }
  .panel {
    width: 760px;
    max-width: calc(100vw - 32px);
    max-height: 72vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-inline-end: 10px;
    border-bottom: 1px solid var(--border);
  }
  .top > input {
    flex: 1;
    height: 44px;
    padding: 0 14px;
    border: 0;
    background: transparent;
    color: var(--fg);
    font-size: 14px;
    outline: none;
    user-select: text;
  }
  .opt {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 6px;
    border-radius: 5px;
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .opt input {
    position: absolute;
    opacity: 0;
    width: 1px;
  }
  .opt:has(input:checked) {
    background: var(--bg-active);
    color: var(--fg);
    border-color: var(--fg-faint);
  }
  .list {
    overflow-y: auto;
    padding: 4px 0;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px 2px;
    font-weight: 600;
    min-width: 0;
  }
  .hit {
    display: flex;
    gap: 10px;
    width: 100%;
    height: 24px;
    align-items: center;
    padding: 0 14px 0 34px;
    border: 0;
    background: transparent;
    text-align: start;
    min-width: 0;
  }
  .hit.active {
    background: var(--bg-hover);
  }
  .ln {
    width: 36px;
    flex-shrink: 0;
    text-align: end;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .txt {
    font-family: var(--font-mono);
    font-size: 12.5px;
    min-width: 0;
  }
  mark {
    background: var(--warn-soft);
    color: inherit;
    outline: 1px solid color-mix(in srgb, var(--warn-line) 50%, transparent);
    border-radius: 2px;
  }
  .foot {
    padding: 6px 14px;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
  }
</style>
