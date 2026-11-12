<script lang="ts">
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import type { OutOfPlan, ParsedPlan, TurnResult, UndoState } from "../types";
  import { Session } from "../stores/session.svelte";
  import { app } from "../stores/app.svelte";
  import FileIcon from "./FileIcon.svelte";

  let {
    session,
    result,
    plan = null,
    outside = [],
    undo = null,
    embedded = false,
    onundo,
  }: {
    session: Session;
    result: TurnResult;
    plan?: ParsedPlan | null;
    outside?: OutOfPlan[];
    undo?: UndoState | null;
    embedded?: boolean;
    onundo: () => void;
  } = $props();

  const changes = $derived(result.changes);
  const files = $derived(changes?.files ?? []);
  const yours = $derived(new Set(result.yourEdits ?? []));

  function word(status: string) {
    return status === "create" ? t("plan.change.create") : status === "delete" ? t("plan.change.delete") : status === "rename" ? t("plan.change.rename") : t("plan.change.edit");
  }

  export function review(index = 0) {
    if (!files.length) return;
    app.viewer = {
      kind: "diff",
      projectId: session.projectId,
      turnId: result.turnId,
      paths: files.map((f) => f.path),
      index,
      sessionKey: session.key,
    };
  }

  function notCoveredText(n: { path: string; why: string }) {
    switch (n.why) {
      case "nestedRepo":
        return t("changes.nestedRepo", { folder: n.path });
      case "bigFile":
        return t("changes.bigFile", { path: n.path });
      case "symlink":
        return t("changes.symlink", { path: n.path });
      default:
        return t("changes.locked", { path: n.path });
    }
  }
</script>

<div class={embedded ? "" : "card"}>
  {#if !embedded}
    <div class="head">
      <span class="font-semibold">{t("changes.title")}</span>
      <span class="sep">.</span>
      <span>{t("changes.files", { count: files.length })}</span>
      {#if files.length}
        <span class="sep">.</span>
        <span class="text-add">+{changes?.added ?? 0}</span>
        <span class="text-del">-{changes?.removed ?? 0}</span>
      {/if}
      <span class="ms-auto text-accent">{t("plan.done")}</span>
    </div>
  {/if}

  {#if files.length === 0 && !changes?.generated.length}
    <p class="px-3 py-2 text-fg-muted">{t("changes.none")}</p>
  {:else}
    <ul class="rows selectable">
      {#each files as f, i (f.path)}
        {@const reason = Session.notInPlan(plan, outside, f.path)}
        <li class:amber={reason !== null}>
          <button class="row" onclick={() => review(i)} title={f.path}>
            <span class="w-14 shrink-0 text-fg-muted">{word(f.status)}</span>
            <FileIcon path={f.path} />
            <span class="path truncate">{#if f.oldPath}<span class="text-fg-muted">{f.oldPath} &rarr; </span>{/if}{f.path}</span>
            <span class="nums">
              {#if f.binary}<span class="text-fg-muted">{t("changes.binary")}</span>
              {:else}
                {#if f.added}<span class="text-add">+{f.added}</span>{/if}
                {#if f.removed}<span class="text-del">-{f.removed}</span>{/if}
              {/if}
            </span>
            {#if yours.has(f.path)}<span class="tag">{t("changes.yourEdit")}</span>{/if}
            {#if reason !== null}
              <span class="note truncate">{reason ? t("plan.notInPlanReason", { reason }) : t("plan.notInPlan")}</span>
            {/if}
          </button>
        </li>
      {/each}
      {#each changes?.generated ?? [] as g (g.folder)}
        <li class="row text-fg-muted">{t("changes.generated", { count: g.count, folder: g.folder })}</li>
      {/each}
    </ul>
  {/if}

  {#if result.notCovered?.length}
    <ul class="px-3 pb-2 text-warn">
      {#each result.notCovered as n (n.path)}
        <li class="truncate">{notCoveredText(n)}</li>
      {/each}
    </ul>
  {/if}
  {#if result.outside}
    <p class="px-3 pb-2 text-warn">{t("changes.outside")}</p>
  {/if}

  <div class="actions">
    {#if undo}
      <span class={undo.outcome.exact ? "text-accent" : "text-warn"}>
        {undo.outcome.exact ? t("changes.undone") : t("changes.undonePartial")}
      </span>
      {#if undo.outcome.kept.length || undo.outcome.failed.length}
        <span class="truncate text-fg-muted">
          {[...undo.outcome.kept, ...undo.outcome.failed.map((f) => f[0])].join(", ")}
        </span>
      {/if}
    {:else}
      {#if files.length}
        <button class="btn" onclick={() => review(0)}>{t("changes.review")} <span class="kbd">R</span></button>
      {/if}
      {#if files.length || changes?.generated.length}
        <button class="btn" onclick={onundo} title={keyLabel("U")}>{t("changes.undo")} <span class="kbd">U</span></button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .card {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    overflow: hidden;
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
  .rows {
    padding: 4px 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: var(--row);
    padding: 0 12px;
    text-align: start;
    background: transparent;
    border: 0;
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  button.row:hover {
    background: var(--bg-hover);
  }
  .path {
    min-width: 0;
    flex: 0 1 auto;
  }
  .nums {
    display: inline-flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .note {
    margin-inline-start: auto;
    font-family: var(--ui-font);
    font-size: 12px;
    color: var(--warn);
    min-width: 0;
  }
  li.amber {
    background: var(--warn-soft);
  }
  .tag {
    font-family: var(--ui-font);
    font-size: 11px;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px 10px;
    min-width: 0;
  }
</style>
