<script lang="ts">
  // One session as a tab: name, agent, and whether it works or waits. Used by the tab strip in
  // the identity bar (Session tabs: Top) and the Sessions view of the side panel (Side).
  // Click shows it, double click renames it, middle click or the close button closes it.
  // Drag reorders (the list uses lib/reorder); Ctrl+Shift+Page Up or Page Down moves the active tab.
  import { Check, TerminalWindow, X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { stateWords, statusBar } from "../stores/statusbar.svelte";
  import type { Session } from "../stores/session.svelte";

  let { session: s, active, closable, vertical = false }: { session: Session; active: boolean; closable: boolean; vertical?: boolean } = $props();

  let editing = $state(false);
  let draft = $state("");
  let input: HTMLInputElement | undefined = $state();

  const name = $derived(s.name || t("session.untitled"));
  const agent = $derived(app.agentName(s.agent));
  // Spinner while working, amber when it waits for you, red when it failed, a check when it is
  // done with changes to review, the dashed dot while dormant (lib/review.ts, tabState).
  const st = $derived(s.tabState);
  const stateText = $derived(st === "idle" ? "" : stateWords(s, statusBar.now));
  const title = $derived([name, s.mode === "pane" ? t("pane.name", { agent }) : agent, stateText].filter(Boolean).join(" . ") + "\n" + t("session.renameHint") + "\n" + t("session.moveHint"));

  function startRename() {
    draft = s.name;
    editing = true;
    queueMicrotask(() => input?.select());
  }
  function commit() {
    if (editing) app.renameSession(s, draft);
    editing = false;
  }
  function close() {
    if (closable) app.closeSession(s.key);
  }
</script>

<div class="tab" class:on={active} class:vertical class:dormant={s.dormant} data-key={s.key}>
  {#if editing}
    <input
      bind:this={input}
      bind:value={draft}
      class="rename"
      aria-label={t("common.rename")}
      onblur={commit}
      onkeydown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") commit();
        if (e.key === "Escape") editing = false;
      }}
    />
  {:else}
    <button
      class="main"
      role="tab"
      aria-selected={active}
      {title}
      onclick={() => app.showSession(s.key)}
      ondblclick={startRename}
      onmousedown={(e) => {
        // Middle click closes; stop the browser's scroll mode.
        if (e.button === 1) e.preventDefault();
      }}
      onauxclick={(e) => {
        if (e.button === 1) close();
      }}
    >
      {#if st === "review"}
        <Check size={11} weight="bold" class="state-check" aria-hidden="true" />
      {:else}
        <span class="state {st}" aria-hidden="true"></span>
      {/if}
      <span class="name truncate">{name}</span>
      <span class="agent">
        {#if s.mode === "pane"}<TerminalWindow size={11} />{/if}
        {agent}
      </span>
      {#if stateText}<span class="sr-only">{stateText}</span>{/if}
    </button>
  {/if}
  {#if closable && !editing}
    <button class="close" title={t("session.close")} aria-label={t("session.close")} onclick={close}>
      <X size={11} />
    </button>
  {/if}
</div>

<style>
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    height: 28px;
    max-width: 240px;
    border-radius: 6px;
    color: var(--fg-muted);
  }
  .tab.vertical {
    max-width: none;
    width: 100%;
    flex-shrink: 1;
  }
  .tab:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .tab.on {
    background: var(--bg-active);
    color: var(--fg);
  }
  .main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
    height: 100%;
    padding: 0 22px 0 8px;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: start;
    white-space: nowrap;
  }
  .name {
    min-width: 0;
    font-weight: 500;
  }
  .vertical .name {
    flex: 1;
  }
  .agent {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    font-size: 11px;
    color: var(--fg-faint);
  }
  .state {
    width: 6px;
    height: 6px;
    flex-shrink: 0;
    border-radius: 50%;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  /* Working: a small spinning ring. */
  .state.working {
    width: 8px;
    height: 8px;
    box-shadow: none;
    border: 1.5px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-top-color: var(--accent);
    animation: spin 0.9s linear infinite;
  }
  .state.waiting {
    background: var(--warn-line);
    box-shadow: none;
  }
  .state.failed {
    background: var(--danger);
    box-shadow: none;
  }
  .state.done {
    background: var(--accent);
    box-shadow: none;
  }
  .main :global(.state-check) {
    flex-shrink: 0;
    color: var(--accent);
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .state.working {
      animation: none;
      background: var(--accent);
      border-color: var(--accent);
    }
  }
  /* A restored tab whose agent has not started: a dashed dot ("Select to resume"). */
  .dormant .state {
    box-shadow: none;
    border: 1px dashed var(--border-strong);
  }
  .close {
    position: absolute;
    inset-inline-end: 3px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-muted);
    opacity: 0;
  }
  .tab:hover .close,
  .close:focus-visible {
    opacity: 1;
  }
  .close:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .rename {
    width: 180px;
    height: 24px;
    margin: 0 2px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    padding: 1px 4px;
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    user-select: text;
  }
  .vertical .rename {
    width: 100%;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
