<script lang="ts">
  import { CheckCircle, Circle, WarningCircle } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import type { Entry } from "../types";
  import { app } from "../stores/app.svelte";
  import { cleanTitle, type Session } from "../stores/session.svelte";
  import ChangesCard from "./ChangesCard.svelte";
  import FileIcon from "./FileIcon.svelte";
  import Markdown from "./Markdown.svelte";

  let { session, entry }: { session: Session; entry: Extract<Entry, { t: "plan" }> } = $props();

  const MAX_STEPS = 6;
  let full = $state(false);
  let card: HTMLDivElement | undefined = $state();
  let changes: ChangesCard | undefined = $state();

  const plan = $derived(entry.plan);
  const steps = $derived(plan?.steps ?? []);
  const files = $derived(plan?.files ?? []);
  const blocked = $derived(app.otherRunning(session));
  const liveLine = $derived.by(() => {
    const doing = session.todo.find((x) => x.status === "in_progress");
    if (doing) return doing.content;
    for (let i = session.entries.length - 1; i >= 0; i--) {
      const e = session.entries[i];
      if (e.t === "tool") return cleanTitle(session.root, e.title, e.paths);
      if (e.t === "plan") break;
    }
    return session.thinking ? t("conv.thinking") : "";
  });

  const status = $derived(
    {
      planning: t("plan.planning"),
      waiting: t("plan.waiting"),
      running: t("plan.running"),
      paused: t("plan.paused"),
      done: t("plan.done"),
      failed: t("plan.failed"),
      discarded: "",
    }[entry.state],
  );

  function run() {
    if (!blocked) app.runPlan(session, entry);
  }
  function discard() {
    app.discardPlan(session, entry);
  }
  function undo() {
    app.askUndo(session, entry);
  }

  /** Single-letter keys work only when the card has focus. */
  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const k = e.key.toLowerCase();
    if (k === "d" && entry.state === "waiting") discard();
    else if (k === "f") full = !full;
    else if (k === "u" && ["done", "paused", "failed"].includes(entry.state) && !entry.undo) undo();
    else if (k === "r" && entry.state === "done") changes?.review(0);
    else if (k === "enter" && entry.state === "waiting") run();
    else return;
    e.preventDefault();
  }

  export function focus() {
    card?.focus();
  }

  function changeWord(c: string) {
    return c === "create" ? t("plan.change.create") : c === "delete" ? t("plan.change.delete") : t("plan.change.edit");
  }
</script>

{#if entry.state === "discarded"}
  <p class="py-1 text-fg-muted">{t("plan.discarded")}</p>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={card}
    class="card"
    class:waiting={entry.state === "waiting"}
    class:failed={entry.state === "failed"}
    tabindex="0"
    role="group"
    aria-label={t("plan.title")}
    data-card
    {onkeydown}
  >
    <div class="head">
      <span class="font-semibold">{t("plan.title")}</span>
      {#if plan}
        <span class="sep">.</span>
        <span>{t("plan.steps", { count: steps.length })}</span>
        <span class="sep">.</span>
        <span>{t("plan.files", { count: files.length })}</span>
      {/if}
      <span
        class="status ms-auto"
        class:text-warn={entry.state === "waiting" || entry.state === "paused"}
        class:pulse-warn={entry.state === "waiting"}
        class:text-accent={entry.state === "done"}
        class:text-danger={entry.state === "failed"}
      >
        {status}
      </span>
    </div>

    {#if entry.state === "planning"}
      <p class="live truncate">{liveLine || t("plan.planning")}</p>
    {:else if entry.state === "done" && entry.result}
      <ChangesCard
        bind:this={changes}
        {session}
        result={entry.result}
        {plan}
        outside={entry.outside}
        undo={entry.undo}
        embedded
        onundo={undo}
      />
    {:else}
      {#if plan?.fallback}
        <p class="px-3 pt-2 text-fg-muted">{t("plan.fallback")}</p>
      {/if}

      {#if full}
        <div class="px-4 py-3"><Markdown text={entry.text} /></div>
      {:else}
        <ol class="steps selectable">
          {#each steps.slice(0, MAX_STEPS) as step, i (i)}
            <li><span class="num">{i + 1}</span><span>{step}</span></li>
          {/each}
          {#if steps.length > MAX_STEPS}
            <li class="text-fg-muted"><span class="num"></span>{t("plan.more", { count: steps.length - MAX_STEPS })}</li>
          {/if}
        </ol>
      {/if}

      {#if files.length || entry.outside.length}
        <ul class="files selectable">
          {#each files as f (f.path)}
            {@const touched = entry.touched.includes(f.path)}
            <li class:muted={entry.state === "running" && !touched}>
              {#if entry.state === "running" || entry.state === "paused"}
                {#if touched}<CheckCircle size={14} weight="fill" class="text-accent" />{:else}<Circle size={14} class="text-fg-faint" />{/if}
              {/if}
              <span class="change">{changeWord(f.change)}</span>
              <FileIcon path={f.path} size={14} />
              <span class="path truncate" title={f.path}>{f.path}</span>
              <span class="reason truncate">{f.found ? f.reason : t("plan.missingFile")}</span>
            </li>
          {/each}
          {#each entry.outside as o (o.path)}
            <li class="amber">
              <WarningCircle size={14} weight="fill" class="text-warn" />
              <span class="change">{t("plan.change.edit")}</span>
              <FileIcon path={o.path} size={14} />
              <span class="path truncate" title={o.path}>{o.path}</span>
              <span class="reason truncate text-warn">
                {o.reason ? t("plan.notInPlanReason", { reason: o.reason }) : t("plan.notInPlan")}
              </span>
              {#if entry.state === "running"}
                <button class="btn btn-warn ms-2 h-6" onclick={() => app.stop(session)}>{t("plan.pause")}</button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}

      {#if plan}
        <p class="risk">{plan.risk && !/^none\.?$/i.test(plan.risk) ? t("plan.risk", { risk: plan.risk }) : t("plan.riskNone")}</p>
      {/if}

      {#if entry.state === "running"}
        <p class="live truncate">{liveLine}</p>
      {/if}

      {#if entry.state === "failed" && entry.result?.error}
        <p class="px-3 pb-2 text-danger selectable">{entry.result.error}</p>
      {/if}

      <div class="actions">
        {#if entry.state === "waiting"}
          <button class="btn btn-primary" onclick={run} disabled={blocked} title={blocked ? t("plan.otherRunning") : ""}>
            {t("plan.run")} <span class="kbd">{keyLabel("Mod+Enter")}</span>
          </button>
          <button class="btn" onclick={discard}>{t("plan.discard")} <span class="kbd">D</span></button>
          {#if blocked}<span class="text-warn">{t("plan.otherRunning")}</span>{/if}
        {:else if entry.state === "running"}
          <button class="btn" onclick={() => app.stop(session)} disabled={session.stopping}>
            {session.stopping ? t("conv.stopping") : t("plan.pause")}
          </button>
        {:else if entry.state === "paused"}
          <button class="btn btn-primary" onclick={() => app.continuePlan(session, entry)} disabled={blocked || session.running}>
            {t("plan.continue")}
          </button>
          {#if entry.undo}
            <span class="text-accent">{entry.undo.outcome.exact ? t("changes.undone") : t("changes.undonePartial")}</span>
          {:else}
            <button class="btn" onclick={undo}>{t("changes.undo")} <span class="kbd">U</span></button>
          {/if}
        {:else if entry.state === "failed"}
          <button class="btn" onclick={() => app.continuePlan(session, entry)} disabled={session.running}>{t("common.retry")}</button>
          {#if entry.result?.hasSnapshot && !entry.undo}
            <button class="btn" onclick={undo}>{t("changes.undo")} <span class="kbd">U</span></button>
          {/if}
        {/if}
        {#if entry.text}
          <button class="btn btn-ghost ms-auto" onclick={() => (full = !full)}>
            {full ? t("plan.hideFull") : t("plan.full")} <span class="kbd">F</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .card {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    overflow: hidden;
    transition: border-color var(--fast) var(--ease);
  }
  .card:focus {
    outline: 2px solid color-mix(in srgb, var(--project) 70%, transparent);
    outline-offset: 2px;
  }
  .card.waiting {
    border-color: color-mix(in srgb, var(--warn-line) 55%, var(--border));
  }
  .card.failed {
    border-color: color-mix(in srgb, var(--danger) 50%, var(--border));
  }
  .head {
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
  .status {
    font-size: 12px;
  }
  .live {
    padding: 8px 12px;
    color: var(--fg-muted);
  }
  .steps {
    padding: 10px 12px 4px;
    font-family: var(--font-read);
    font-size: 14px;
    line-height: 1.5;
  }
  .steps li {
    display: flex;
    gap: 10px;
    padding: 2px 0;
  }
  .num {
    width: 14px;
    flex-shrink: 0;
    text-align: end;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .files {
    padding: 6px 0;
    margin-top: 4px;
  }
  .files li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: var(--row);
    padding: 0 12px;
    transition: opacity var(--fast) var(--ease);
  }
  .files li.muted {
    opacity: 0.6;
  }
  .files li.amber {
    background: var(--warn-soft);
  }
  .change {
    width: 44px;
    flex-shrink: 0;
    color: var(--fg-muted);
  }
  .path {
    font-family: var(--font-mono);
    font-size: 12.5px;
    flex: 0 1 auto;
    min-width: 80px;
    max-width: 50%;
  }
  .reason {
    color: var(--fg-muted);
    margin-inline-start: 8px;
    flex: 1 1 0;
    min-width: 0;
  }
  .risk {
    padding: 4px 12px 0;
    color: var(--fg-muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
  }
</style>
