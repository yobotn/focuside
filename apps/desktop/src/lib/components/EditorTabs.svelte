<script lang="ts">
  // The tab strip of the viewer: one tab per open file, plus the diff tab of a turn review.
  // Click shows a tab, double click pins a preview tab (italic), middle click or the close
  // button closes it, drag reorders (and pins). Right click, Shift+F10 or the menu key opens
  // the tab menu. Left and Right arrows move between tabs, Delete closes one.
  import { GitDiff, X } from "phosphor-svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { t } from "../i18n";
  import { isWindows, keyLabel } from "../platform";
  import { reorder } from "../reorder";
  import { app, type Project } from "../stores/app.svelte";
  import { DIFF_KEY, editor, tabKey, tabNames, type Tab, type Workspace } from "../stores/editor.svelte";
  import FileIcon from "./FileIcon.svelte";

  let { project, w }: { project: Project; w: Workspace } = $props();

  const names = $derived(tabNames(w.tabs));
  let strip: HTMLDivElement | undefined = $state();
  let menu = $state<{ key: string; x: number; y: number } | null>(null);
  let menuEl: HTMLDivElement | undefined = $state();

  // Keep the active tab in view.
  $effect(() => {
    const key = w.active;
    if (!key || !strip) return;
    queueMicrotask(() => strip?.querySelector(`[data-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" }));
  });

  function label(tab: Tab): string {
    if (tab.kind === "diff") return t("editor.diffTab", { name: tab.paths[tab.index]?.split("/").pop() ?? "" });
    return names.get(tab.path)?.name ?? tab.path;
  }

  function title(tab: Tab): string {
    if (tab.kind === "diff") return `${t("viewer.diff")}: ${tab.paths[tab.index] ?? ""}`;
    const lines = [tab.path];
    if (w.docs[tab.path]?.dirty) lines.push(t("viewer.unsaved"));
    if (tab.preview) lines.push(t("editor.previewHint"));
    return lines.join("\n");
  }

  function close(key: string) {
    editor.close(w, [key]);
  }

  function onkey(e: KeyboardEvent, i: number) {
    if (e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    let next = -1;
    if (e.key === "ArrowRight") next = (i + 1) % w.tabs.length;
    else if (e.key === "ArrowLeft") next = (i - 1 + w.tabs.length) % w.tabs.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = w.tabs.length - 1;
    else if (e.key === "Delete") {
      e.preventDefault();
      close(tabKey(w.tabs[i]));
      return;
    } else return;
    e.preventDefault();
    const key = tabKey(w.tabs[next]);
    editor.activate(w, key);
    queueMicrotask(() => strip?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"] [role=tab]`)?.focus());
  }

  function openMenu(e: MouseEvent, key: string) {
    e.preventDefault();
    let x = e.clientX;
    let y = e.clientY;
    if (!x && !y) {
      // From the keyboard: below the tab.
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      x = r.left;
      y = r.bottom + 2;
    }
    menu = { key, x: Math.min(x, window.innerWidth - 228), y };
    queueMicrotask(() => {
      if (!menuEl) return;
      const h = menuEl.offsetHeight;
      if (menu && menu.y + h > window.innerHeight - 8) menu = { ...menu, y: Math.max(8, menu.y - h) };
      menuEl.querySelector<HTMLElement>("[role=menuitem]")?.focus();
    });
  }

  function hideMenu(refocus = true) {
    const key = menu?.key;
    menu = null;
    if (refocus && key) strip?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"] [role=tab]`)?.focus();
  }

  function absPath(rel: string): string {
    const sep = isWindows ? "\\" : "/";
    const root = project.root.replace(/[\\/]+$/, "");
    return root + sep + (isWindows ? rel.replace(/\//g, "\\") : rel);
  }

  async function copy(text: string) {
    try {
      await writeText(text);
    } catch {
      await navigator.clipboard?.writeText(text).catch(() => {});
    }
    app.flash(t("common.copied"));
  }

  const menuTab = $derived(menu ? (w.tabs[w.index(menu.key)] ?? null) : null);
  const items = $derived.by(() => {
    if (!menu || !menuTab) return [];
    const key = menu.key;
    const i = w.index(key);
    const file = menuTab.kind === "file" ? menuTab.path : null;
    const list: ({ label: string; hint?: string; run: () => void; disabled?: boolean } | null)[] = [
      { label: t("editor.close"), hint: keyLabel("Mod+W"), run: () => close(key) },
      { label: t("editor.closeOthers"), run: () => editor.closeOthers(w, key), disabled: w.tabs.length < 2 },
      { label: t("editor.closeRight"), run: () => editor.closeRight(w, key), disabled: i >= w.tabs.length - 1 },
      { label: t("editor.closeSaved"), run: () => editor.closeSaved(w) },
    ];
    if (file) {
      list.push(
        null,
        { label: t("editor.copyPath"), run: () => copy(absPath(file)) },
        { label: t("editor.copyRelative"), run: () => copy(file) },
        null,
        { label: t("editor.reveal"), run: () => editor.requestReveal(project.id, file) },
      );
      if (menuTab.preview) list.push({ label: t("editor.keepOpen"), run: () => editor.pin(w, key) });
    }
    return list;
  });

  function onmenukey(e: KeyboardEvent) {
    const all = [...(menuEl?.querySelectorAll<HTMLElement>("[role=menuitem]:not([disabled])") ?? [])];
    const i = all.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") all[(i + 1) % all.length]?.focus();
    else if (e.key === "ArrowUp") all[(i - 1 + all.length) % all.length]?.focus();
    else if (e.key === "Home") all[0]?.focus();
    else if (e.key === "End") all[all.length - 1]?.focus();
    else if (e.key === "Escape") hideMenu();
    else if (e.key === "Tab") hideMenu(false);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (menu && !menuEl?.contains(e.target as Node)) hideMenu(false);
  }}
/>

<div
  class="tabs"
  role="tablist"
  aria-label={t("editor.tabs")}
  bind:this={strip}
  onwheel={(e) => {
    if (strip && Math.abs(e.deltaY) > Math.abs(e.deltaX)) strip.scrollLeft += e.deltaY;
  }}
  {@attach reorder("x", (key, i) => editor.move(w, key, i))}
>
  {#each w.tabs as tab, i (tabKey(tab))}
    {@const key = tabKey(tab)}
    {@const on = key === w.active}
    {@const dirty = tab.kind === "file" && !!w.docs[tab.path]?.dirty}
    {@const gone = tab.kind === "file" && w.docs[tab.path]?.content?.kind === "missing"}
    {@const dir = tab.kind === "file" ? names.get(tab.path)?.dir : ""}
    <div class="tab" class:on class:preview={tab.preview} class:dirty data-key={key}>
      <button
        class="main"
        role="tab"
        aria-selected={on}
        tabindex={on ? 0 : -1}
        title={title(tab)}
        onclick={() => editor.activate(w, key)}
        ondblclick={() => editor.pin(w, key)}
        onmousedown={(e) => {
          if (e.button === 1) e.preventDefault();
        }}
        onauxclick={(e) => {
          if (e.button === 1) close(key);
        }}
        oncontextmenu={(e) => openMenu(e, key)}
        onkeydown={(e) => onkey(e, i)}
      >
        {#if tab.kind === "diff"}
          <GitDiff size={14} class="shrink-0 text-fg-muted" />
        {:else}
          <FileIcon path={tab.path} size={14} />
        {/if}
        <span class="name truncate" class:gone>{label(tab)}</span>
        {#if dir}<span class="dir truncate">{dir}</span>{/if}
        {#if dirty}<span class="sr-only">{t("viewer.unsaved")}</span>{/if}
      </button>
      <button
        class="close"
        tabindex="-1"
        title="{t('editor.close')} ({keyLabel('Mod+W')})"
        aria-label={t("editor.closeName", { name: tab.kind === "file" ? tab.path : label(tab) })}
        onclick={() => close(key)}
      >
        <span class="unsaved" aria-hidden="true"></span>
        <X size={11} class="x" />
      </button>
    </div>
  {/each}
</div>

{#if menu}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div bind:this={menuEl} class="menu" role="menu" tabindex="-1" aria-label={t("editor.tabMenu")} style="left: {menu.x}px; top: {menu.y}px" onkeydown={onmenukey}>
    {#each items as it, i (i)}
      {#if it}
        <button
          role="menuitem"
          tabindex="-1"
          class="item"
          disabled={it.disabled}
          onclick={() => {
            hideMenu(false);
            it.run();
          }}
        >
          <span class="flex-1">{it.label}</span>
          {#if it.hint}<span class="kbd">{it.hint}</span>{/if}
        </button>
      {:else}
        <div class="sep" role="separator"></div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .tabs {
    display: flex;
    align-items: stretch;
    height: 100%;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    max-width: 220px;
    border-inline-end: 1px solid var(--border);
    color: var(--fg-muted);
    background: var(--bg-subtle);
  }
  .tab:hover {
    color: var(--fg);
  }
  .tab.on {
    background: var(--bg);
    color: var(--fg);
    box-shadow: inset 0 2px 0 var(--project);
  }
  .main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 100%;
    padding: 0 26px 0 10px;
    border: 0;
    background: transparent;
    color: inherit;
    white-space: nowrap;
  }
  .main:focus-visible {
    outline-offset: -2px;
  }
  .name {
    min-width: 0;
  }
  .preview .name {
    font-style: italic;
  }
  .name.gone {
    text-decoration: line-through;
  }
  .dir {
    min-width: 0;
    max-width: 90px;
    font-size: 11px;
    color: var(--fg-faint);
  }
  .close {
    position: absolute;
    inset-inline-end: 4px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-muted);
  }
  .close :global(.x) {
    grid-area: 1 / 1;
    opacity: 0;
  }
  .unsaved {
    grid-area: 1 / 1;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--fg-muted);
    opacity: 0;
  }
  .dirty .unsaved {
    opacity: 1;
  }
  .tab:hover .close :global(.x),
  .tab.on:not(.dirty) .close :global(.x),
  .close:focus-visible :global(.x) {
    opacity: 1;
  }
  .tab:hover .unsaved,
  .close:focus-visible .unsaved {
    opacity: 0;
  }
  .close:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .menu {
    position: fixed;
    z-index: 70;
    width: 220px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    box-shadow: var(--shadow);
    outline: none;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--fg);
    text-align: start;
  }
  .item:hover:not(:disabled),
  .item:focus-visible {
    background: var(--bg-hover);
    outline: none;
  }
  .item:disabled {
    color: var(--fg-faint);
  }
  .sep {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }
</style>
