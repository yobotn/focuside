<script lang="ts">
  // The plan panel. It shows, for one session (the active one unless `sessionKey` is given):
  // - the newest plan (a pane: ExitPlanMode from hooks; chat: the plan card text),
  // - the todo list (a pane: TodoWrite or the Task tools from hooks; chat: the agent's plan
  //   updates) with a mark for each status,
  // - one line per turn ("3 files changed") with Undo. The file list opens on click.
  // All data comes from the session store. The panel has no required props, so the layout can
  // place it anywhere (a dock tab next to the file viewer by default).
  import { CaretRight, CheckCircle, Circle, CircleHalf } from "phosphor-svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";
  import type { Entry, TurnResult, UndoState } from "../types";
  import FileIcon from "./FileIcon.svelte";
  import Markdown from "./Markdown.svelte";

  let { sessionKey }: { sessionKey?: string } = $props();

  const session = $derived<Session | undefined>(sessionKey ? app.sessionFor(sessionKey) : app.session);
  const plan = $derived(session?.latestPlan ?? null);
  const todos = $derived(session?.todos ?? []);
  const done = $derived(todos.filter((x) => x.status === "completed").length);

  type TurnEntry = Extract<Entry, { t: "changes" }> | Extract<Entry, { t: "plan" }>;
  type Line = { entry: TurnEntry; result: TurnResult; undo: UndoState | null } | { note: Extract<Entry, { t: "note" }> };

  // Newest first. Pane notes (undo, untracked prompt, API error) sit between the turns; chat
  // notes stay in the conversation.
  const lines = $derived.by<Line[]>(() => {
    const s = session;
    if (!s) return [];
    const out: Line[] = [];
    for (const e of s.entries) {
      if (e.t === "changes") out.push({ entry: e, result: e.result, undo: e.undo });
      else if (e.t === "plan" && e.result?.changes && (e.result.changes.files.length || e.result.changes.generated.length))
        out.push({ entry: e, result: e.result, undo: e.undo });
      else if (e.t === "note" && s.mode === "pane" && e.tone !== "muted") out.push({ note: e });
    }
    return out.reverse();
  });

  const empty = $derived(!plan && !todos.length && !lines.length);
  // A pane without hooks has no plan, todos or turns. One quiet line says why.
  const trackingOff = $derived(!!session && session.mode === "pane" && !session.hooksExpected && !session.hooksLive);

  let open = $state<Record<string, boolean>>({});

  function review(s: Session, r: TurnResult, index: number) {
    const files = r.changes?.files ?? [];
    if (!files.length) return;
    app.viewer = { kind: "diff", projectId: s.projectId, turnId: r.turnId, paths: files.map((f) => f.path), index, sessionKey: s.key };
  }

  function statusLabel(status: string) {
    return status === "completed" ? t("planPanel.todoDone") : status === "in_progress" ? t("planPanel.todoActive") : t("planPanel.todoOpen");
  }
</script>

<section class="panel" aria-label={t("planPanel.title")}>
  {#if !session || empty}
    <div class="empty text-fg-muted">
      <p>{t("planPanel.empty")}</p>
      {#if trackingOff}<p class="quiet">{t("pane.trackingOff")}</p>{/if}
    </div>
  {:else}
    {#if plan}
      <div class="block">
        <div class="head">
          <span class="font-semibold">{t("planPanel.title")}</span>
          {#if plan.approved !== null}
            <span class="ms-auto" class:text-accent={plan.approved} class:text-warn={!plan.approved}>
              {plan.approved ? t("pane.planApproved") : t("pane.planProposed")}
            </span>
          {/if}
        </div>
        <Markdown class="plan-text selectable" text={plan.text} />
        {#if plan.approved === false}<p class="text-fg-muted">{t("pane.planHint")}</p>{/if}
      </div>
    {/if}

    {#if todos.length}
      <div class="block">
        <div class="head">
          <span class="font-semibold">{t("planPanel.todos")}</span>
          <span class="ms-auto text-fg-muted">{t("planPanel.todosDone", { done, count: todos.length })}</span>
        </div>
        <ul class="todos selectable">
          {#each todos as item, i (i)}
            <li class:done={item.status === "completed"} class:active={item.status === "in_progress"}>
              <span class="mark" title={statusLabel(item.status)}>
                {#if item.status === "completed"}<CheckCircle size={15} weight="fill" />
                {:else if item.status === "in_progress"}<CircleHalf size={15} weight="fill" />
                {:else}<Circle size={15} />{/if}
                <span class="sr-only">{statusLabel(item.status)}</span>
              </span>
              <span class="text">{item.content}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if lines.length}
      <div class="block">
        <div class="head"><span class="font-semibold">{t("planPanel.changes")}</span></div>
        <ul class="turns">
          {#each lines as line ("note" in line ? line.note.id : line.entry.id)}
            {#if "note" in line}
              <li class="note" class:text-warn={line.note.tone === "warn"} class:text-danger={line.note.tone === "error"}>{line.note.text}</li>
            {:else}
              {@const files = line.result.changes?.files ?? []}
              {@const id = line.entry.id}
              <li>
                <div class="turn">
                  <button class="toggle" aria-expanded={!!open[id]} disabled={!files.length} onclick={() => (open[id] = !open[id])}>
                    <span class="caret" class:rot={open[id]}><CaretRight size={12} /></span>
                    <span class="truncate">{files.length ? t("planPanel.turn", { count: files.length }) : t("planPanel.generated")}</span>
                    {#if files.length}
                      <span class="text-add">+{line.result.changes?.added ?? 0}</span>
                      <span class="text-del">-{line.result.changes?.removed ?? 0}</span>
                    {/if}
                  </button>
                  {#if line.undo}
                    <span class="state" class:text-accent={line.undo.outcome.exact} class:text-warn={!line.undo.outcome.exact}>{t("planPanel.undone")}</span>
                  {:else if session}
                    {@const s = session}
                    {@const entry = line.entry}
                    <button class="btn h-6" onclick={() => app.askUndo(s, entry)}>{t("changes.undo")}</button>
                  {/if}
                </div>
                {#if open[id] && session}
                  {@const s = session}
                  <ul class="files">
                    {#each files as f, i (f.path)}
                      <li>
                        <button class="file" title={f.path} onclick={() => review(s, line.result, i)}>
                          <FileIcon path={f.path} />
                          <span class="truncate">{f.path}</span>
                          {#if f.added}<span class="text-add">+{f.added}</span>{/if}
                          {#if f.removed}<span class="text-del">-{f.removed}</span>{/if}
                        </button>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </li>
            {/if}
          {/each}
        </ul>
      </div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 14px;
    font-size: 13px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .quiet {
    font-size: 12px;
    color: var(--fg-faint);
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
  }
  .todos li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 2px 0;
  }
  .mark {
    display: inline-flex;
    padding-top: 2px;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .todos li.active .mark {
    color: var(--warn);
  }
  .todos li.done .mark {
    color: var(--accent);
  }
  .todos li.done .text {
    color: var(--fg-muted);
    text-decoration: line-through;
  }
  .turns {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .turn {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    height: var(--row);
    padding: 0 4px;
    background: transparent;
    border: 0;
    border-radius: 4px;
    text-align: start;
  }
  .toggle:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .caret {
    display: inline-flex;
    color: var(--fg-muted);
    transition: transform 120ms;
  }
  .caret.rot {
    transform: rotate(90deg);
  }
  .state {
    font-size: 12px;
  }
  .note {
    padding: 2px 4px;
    font-size: 12.5px;
  }
  .files {
    padding: 2px 0 4px 18px;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    height: var(--row);
    padding: 0 4px;
    background: transparent;
    border: 0;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12.5px;
    text-align: start;
  }
  .file:hover {
    background: var(--bg-hover);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  :global(.plan-text) {
    font-size: 13px;
  }
</style>
