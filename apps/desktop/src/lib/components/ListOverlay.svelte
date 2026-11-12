<script lang="ts" module>
  export interface ListItem {
    id: string;
    label: string;
    detail?: string;
    hint?: string;
    /** Icon file path (file icon) or a project color hue for a dot. */
    file?: string;
    hue?: number;
    tone?: "warn" | "muted";
    run: () => void;
  }
</script>

<script lang="ts">
  import { t } from "../i18n";
  import { filter } from "../fuzzy";
  import FileIcon from "./FileIcon.svelte";

  let {
    title = "",
    placeholder = "",
    items,
    empty = t("palette.empty"),
    searchable = true,
    mono = false,
    onclose,
  }: {
    title?: string;
    placeholder?: string;
    items: ListItem[];
    empty?: string;
    searchable?: boolean;
    mono?: boolean;
    onclose: () => void;
  } = $props();

  let query = $state("");
  let index = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLUListElement | undefined = $state();
  let panel: HTMLDivElement | undefined = $state();

  const shown = $derived(searchable ? filter(items, query, (i) => i.label + (i.detail ? " " + i.detail : ""), 200) : items);

  $effect(() => {
    void query;
    index = 0;
  });
  $effect(() => {
    list?.children[index]?.scrollIntoView({ block: "nearest" });
  });
  $effect(() => {
    queueMicrotask(() => (input ?? panel)?.focus());
  });

  function pick(i: number) {
    const it = shown[i];
    if (!it) return;
    onclose();
    it.run();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") index = Math.min(shown.length - 1, index + 1);
    else if (e.key === "ArrowUp") index = Math.max(0, index - 1);
    else if (e.key === "Enter") pick(index);
    else if (e.key === "Escape") onclose();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="panel" bind:this={panel} tabindex="-1" onclick={(e) => e.stopPropagation()} {onkeydown} role="dialog" aria-label={title || placeholder}>
    {#if searchable}
      <input bind:this={input} bind:value={query} {placeholder} class="search" spellcheck="false" />
    {:else if title}
      <p class="title">{title}</p>
    {/if}
    {#if shown.length === 0}
      <p class="px-4 py-3 text-fg-muted">{empty}</p>
    {:else}
      <ul bind:this={list} class="list">
        {#each shown as it, i (it.id)}
          <li>
            <button class="item" class:active={i === index} onclick={() => pick(i)} onmousemove={() => (index = i)}>
              {#if it.file}<FileIcon path={it.file} size={14} />{/if}
              {#if it.hue !== undefined}<span class="hue" style="background: hsl({it.hue} var(--project-s) var(--project-l))"></span>{/if}
              <span class="truncate" class:mono class:text-warn={it.tone === "warn"}>{it.label}</span>
              {#if it.detail}<span class="detail truncate">{it.detail}</span>{/if}
              {#if it.hint}<span class="kbd ms-auto shrink-0 ps-3">{it.hint}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
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
    padding-top: 12vh;
  }
  .panel {
    width: 600px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
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
  .title {
    padding: 12px 14px 8px;
    font-weight: 600;
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
  }
  .item.active {
    background: var(--bg-hover);
  }
  .detail {
    color: var(--fg-muted);
    min-width: 0;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .hue {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
