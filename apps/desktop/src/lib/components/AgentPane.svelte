<script lang="ts">
  // Pane mode: the agent's own unmodified TUI (for example `claude`) in a terminal in the
  // center view. The user types every prompt there. FocusIDE shows the changes of each turn
  // beside it (or above it in a narrow window), with Undo turn. A turn ends on Checkpoint or
  // when the TUI exits. FocusIDE never types into the pane (see stores/paneterm).
  import { onDestroy, onMount } from "svelte";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import { paneTerms, type PaneTerm } from "../stores/paneterm.svelte";
  import type { Session } from "../stores/session.svelte";
  import ChangesCard from "./ChangesCard.svelte";

  let { session, project }: { session: Session; project: Project } = $props();

  let host: HTMLDivElement | undefined = $state();
  let cardsEl: HTMLElement | undefined = $state();
  let width = $state(1200);
  let pane: PaneTerm | null = $state(null);
  let ro: ResizeObserver | null = null;

  // Cards beside the terminal when there is room for both; above it otherwise.
  const side = $derived(width >= 980);
  const name = $derived(app.agentName(session.agent));
  const note = $derived(session.agent === "claude" ? t("pane.noteClaude") : t("pane.note", { agent: name }));
  const cards = $derived(session.entries.filter((e) => e.t === "changes" || e.t === "note"));

  const status = $derived.by(() => {
    if (pane?.exited || (!session.paneAlive && session.status === "ready")) return { text: t("pane.exited", { agent: name }), tone: "muted" };
    switch (session.tracking) {
      case "on":
        return { text: t("pane.tracking"), tone: "muted" };
      case "otherRunning":
        return { text: t("pane.notTracked"), tone: "warn" };
      case "error":
        return { text: t("pane.trackingFailed", { error: session.trackingError ?? "" }), tone: "warn" };
      default:
        return { text: session.paneAlive ? t("pane.trackingOff") : t("session.starting", { agent: name }), tone: "muted" };
    }
  });

  onMount(() => {
    pane = paneTerms.get(session.key, project.id);
    pane.attach(host!);
    ro = new ResizeObserver(() => pane?.resize());
    ro.observe(host!);
  });

  onDestroy(() => {
    ro?.disconnect();
    // The TUI keeps running while another session is shown.
    pane?.detach();
  });

  // Newest card in view.
  $effect(() => {
    void cards.length;
    queueMicrotask(() => cardsEl?.scrollTo({ top: cardsEl.scrollHeight }));
  });
</script>

<div class="view" class:side bind:clientWidth={width}>
  <div class="bar">
    <span class="note truncate" title={note}>{note}</span>
    <span class="flex-1"></span>
    <span class="status truncate" class:text-warn={status.tone === "warn"} title={status.text}>{status.text}</span>
    <button
      class="btn"
      disabled={!session.paneAlive}
      title={t("pane.checkpointHint")}
      onclick={() => {
        app.checkpoint(session);
        pane?.focus();
      }}
    >
      {t("pane.checkpoint")}
    </button>
  </div>
  <div class="body">
    <div class="term" bind:this={host}></div>
    {#if side || cards.length}
      <aside class="cards" bind:this={cardsEl} aria-label={t("pane.cards")}>
        {#if !cards.length}
          <p class="text-fg-muted">{t("pane.cardsEmpty")}</p>
        {/if}
        {#each cards as entry (entry.id)}
          {#if entry.t === "changes"}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
            <div
              tabindex="0"
              data-card
              role="group"
              aria-label={t("changes.title")}
              class="card-wrap rounded-lg"
              onkeydown={(e) => {
                if (e.ctrlKey || e.metaKey || e.altKey) return;
                if (e.key.toLowerCase() === "u" && !entry.undo) app.askUndo(session, entry);
              }}
            >
              <ChangesCard {session} result={entry.result} undo={entry.undo} onundo={() => app.askUndo(session, entry)} />
            </div>
          {:else if entry.t === "note"}
            <p class="selectable" class:text-fg-muted={entry.tone === "muted"} class:text-danger={entry.tone === "error"} class:text-warn={entry.tone === "warn"}>
              {entry.text}
            </p>
          {/if}
        {/each}
      </aside>
    {/if}
  </div>
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 0 10px 0 14px;
    border-bottom: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 12.5px;
  }
  .note {
    min-width: 0;
    flex: 0 1 auto;
  }
  .status {
    min-width: 0;
    flex: 0 1 auto;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column-reverse;
  }
  .side .body {
    flex-direction: row;
  }
  .term {
    flex: 1;
    min-width: 0;
    min-height: 120px;
    padding: 6px 0 4px 10px;
    background: var(--bg-sunken);
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    padding: 12px;
    max-height: 38%;
    border-bottom: 1px solid var(--border);
  }
  .side .cards {
    width: 380px;
    flex-shrink: 0;
    max-height: none;
    border-bottom: 0;
    border-inline-start: 1px solid var(--border);
  }
  .card-wrap:focus {
    outline: 2px solid color-mix(in srgb, var(--project) 70%, transparent);
    outline-offset: 2px;
  }
</style>
