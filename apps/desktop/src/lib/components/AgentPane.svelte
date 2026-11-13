<script lang="ts">
  // Pane mode: the agent's own unmodified TUI (for example `claude`) in a terminal in the
  // center view. The user types every prompt there. FocusIDE shows the changes of each turn
  // beside it (or above it in a narrow window), with Undo turn and a short summary. With hooks
  // (the focuside plugin, or hooks the user added), a prompt starts a turn and the agent's stop
  // ends it, and a proposed plan shows as a card. Without hooks a turn ends on Checkpoint or
  // when the TUI exits. FocusIDE never types into the pane (see stores/paneterm).
  import { onDestroy, onMount } from "svelte";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import { paneTerms, type PaneTerm } from "../stores/paneterm.svelte";
  import type { Session } from "../stores/session.svelte";
  import ChangesCard from "./ChangesCard.svelte";
  import FileIcon from "./FileIcon.svelte";

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
  const cards = $derived(session.entries.filter((e) => e.t === "changes" || e.t === "note" || e.t === "panePlan"));
  const hookMode = $derived(session.hooksLive || session.hooksExpected);
  const MAX_STEPS = 6;

  const status = $derived.by(() => {
    if (pane?.exited || (!session.paneAlive && session.status === "ready")) return { text: t("pane.exited", { agent: name }), tone: "muted" };
    switch (session.tracking) {
      case "on":
        return { text: session.turnSource === "hook" ? t("pane.trackingHook", { agent: name }) : t("pane.tracking"), tone: "muted" };
      case "otherRunning":
        return { text: hookMode ? t("pane.notTrackedHook") : t("pane.notTracked"), tone: "warn" };
      case "waiting":
        return { text: session.hooksLive ? t("pane.hooksWaiting") : t("pane.hooksExpected"), tone: "muted" };
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
      title={session.hooksLive ? t("pane.checkpointHintHooks") : t("pane.checkpointHint")}
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
          <p class="text-fg-muted">{hookMode ? t("pane.cardsEmptyHooks") : t("pane.cardsEmpty")}</p>
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
              <ChangesCard
                {session}
                result={entry.result}
                plan={session.panePlanOf(entry.turnId)}
                undo={entry.undo}
                onundo={() => app.askUndo(session, entry)}
              />
            </div>
          {:else if entry.t === "panePlan"}
            <section class="plan" aria-label={t("plan.title")}>
              <div class="plan-head">
                <span class="font-semibold">{t("plan.title")}</span>
                <span class="sep">.</span>
                <span>{t("plan.steps", { count: entry.plan.steps.length })}</span>
                {#if entry.plan.files.length}
                  <span class="sep">.</span>
                  <span>{t("plan.files", { count: entry.plan.files.length })}</span>
                {/if}
                <span class="ms-auto" class:text-accent={entry.approved} class:text-warn={!entry.approved}>
                  {entry.approved ? t("pane.planApproved") : t("pane.planProposed")}
                </span>
              </div>
              <div class="plan-body selectable">
                {#if entry.plan.fallback}<p class="text-fg-muted">{t("plan.fallback")}</p>{/if}
                <ol>
                  {#each entry.plan.steps.slice(0, MAX_STEPS) as step, i (i)}<li>{step}</li>{/each}
                </ol>
                {#if entry.plan.steps.length > MAX_STEPS}
                  <p class="text-fg-muted">{t("plan.more", { count: entry.plan.steps.length - MAX_STEPS })}</p>
                {/if}
                {#if entry.plan.files.length}
                  <ul class="files">
                    {#each entry.plan.files as f (f.path)}
                      <li title={f.reason || f.path}>
                        <span class="w-12 shrink-0 text-fg-muted">{t(`plan.change.${f.change}`)}</span>
                        <FileIcon path={f.path} />
                        <span class="truncate">{f.path}</span>
                        {#if !f.found}<span class="text-warn">{t("plan.missingFile")}</span>{/if}
                      </li>
                    {/each}
                  </ul>
                {/if}
                <p class="text-fg-muted">{entry.plan.risk ? t("plan.risk", { risk: entry.plan.risk }) : t("plan.riskNone")}</p>
                {#if !entry.approved}<p class="text-fg-muted">{t("pane.planHint")}</p>{/if}
              </div>
            </section>
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
  .plan {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    overflow: hidden;
  }
  .plan-head {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }
  .sep {
    color: var(--fg-faint);
  }
  .plan-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px 10px;
  }
  .plan-body ol {
    padding-inline-start: 20px;
    list-style: decimal;
  }
  .files li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .card-wrap:focus {
    outline: 2px solid color-mix(in srgb, var(--project) 70%, transparent);
    outline-offset: 2px;
  }
</style>
