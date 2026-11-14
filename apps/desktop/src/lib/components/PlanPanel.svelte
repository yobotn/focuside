<script lang="ts">
  // The plan panel. It shows, for one session (the active one unless `sessionKey` is given):
  // - the plan history (`Session.planVersions`, kept in app data): every plan (a pane:
  //   ExitPlanMode from hooks; chat: the plan card text) and every todo list whose items
  //   changed. A new version opens at the newest version with its changes against the version
  //   before marked (added lines highlighted, removed lines struck through; "Show changes"
  //   turns it off). A compact selector picks an older version,
  // - the todo list (a pane: TodoWrite or the Task tools from hooks; chat: the agent's plan
  //   updates) with a mark for each status,
  // - one line per turn ("3 files changed") with Undo. The file list opens on click.
  // All data comes from the session store. The panel has no required props, so the layout can
  // place it anywhere (a dock tab next to the file viewer by default).
  import { CaretRight, CheckCircle, Circle, CircleHalf } from "phosphor-svelte";
  import { t } from "../i18n";
  import { diffLines } from "../linediff";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";
  import type { Entry, PlanVersion, TurnResult, UndoState } from "../types";
  import FileIcon from "./FileIcon.svelte";
  import Markdown from "./Markdown.svelte";

  let { sessionKey }: { sessionKey?: string } = $props();

  const session = $derived<Session | undefined>(sessionKey ? app.sessionFor(sessionKey) : app.session);
  const plan = $derived(session?.latestPlan ?? null);
  const versions = $derived(session?.planVersions ?? []);
  /** The version the user picked; null follows the newest. */
  let picked = $state<number | null>(null);
  let showChanges = $state(true);
  let seenKey: string | undefined;
  let seenLen = 0;
  // A new version, or another session: back to the newest version with its changes shown.
  $effect(() => {
    const key = session?.key;
    const n = versions.length;
    if (key !== seenKey || n > seenLen) {
      picked = null;
      showChanges = true;
    }
    seenKey = key;
    seenLen = n;
  });
  const current = $derived<PlanVersion | null>(versions.find((v) => v.n === picked) ?? versions[versions.length - 1] ?? null);
  const previous = $derived(current ? versions.findLast((v) => v.source === current.source && v.n < current.n) : undefined);
  const diff = $derived(current && previous && showChanges ? diffLines(previous.text, current.text) : null);
  const liveTodos = $derived(session?.todos ?? []);
  // A todo list version shows its own items; the newest one keeps the live statuses.
  const todos = $derived(current?.source === "todos" ? (current.todos ?? []) : liveTodos);
  const done = $derived(todos.filter((x) => x.status === "completed").length);

  function versionLabel(v: PlanVersion): string {
    const time = new Date(v.at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    return v.source === "todos" ? t("planPanel.versionTodos", { n: v.n, time }) : t("planPanel.version", { n: v.n, time });
  }

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

  const empty = $derived(!plan && !versions.length && !todos.length && !lines.length);
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
    {#if current}
      <div class="block">
        <div class="head">
          <span class="font-semibold">{current.source === "todos" ? t("planPanel.todos") : t("planPanel.title")}</span>
          {#if current.approved !== null}
            <span class:text-accent={current.approved} class:text-warn={!current.approved}>
              {current.approved ? t("pane.planApproved") : t("pane.planProposed")}
            </span>
          {/if}
          <span class="flex-1"></span>
          {#if versions.length > 1}
            <select
              class="versions"
              aria-label={t("planPanel.versions")}
              value={String(current.n)}
              onchange={(e) => {
                const n = Number(e.currentTarget.value);
                picked = n === versions[versions.length - 1].n ? null : n;
              }}
            >
              {#each [...versions].reverse() as v (v.n)}<option value={String(v.n)}>{versionLabel(v)}</option>{/each}
            </select>
          {/if}
        </div>
        {#if previous}
          <label class="changes-toggle text-fg-muted">
            <input type="checkbox" bind:checked={showChanges} />
            {t("planPanel.showChanges", { n: previous.n })}
          </label>
        {/if}
        {#if diff}
          <div class="diff selectable" role="list" aria-label={t("planPanel.changesSince", { n: previous?.n ?? 0 })}>
            {#each diff as line, i (i)}
              <div class="dl {line.kind}" role="listitem">
                {#if line.kind === "add"}<span class="sr-only">{t("planPanel.added")}</span>{:else if line.kind === "del"}<span class="sr-only">{t("planPanel.removed")}</span>{/if}
                {line.text || " "}
              </div>
            {/each}
          </div>
        {:else if current.source === "plan"}
          <Markdown class="plan-text selectable" text={current.text} />
        {/if}
        {#if current.source === "plan" && current.approved === false && !picked}<p class="text-fg-muted">{t("pane.planHint")}</p>{/if}
      </div>
    {:else if plan}
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

    {#if todos.length && !(current?.source === "todos" && diff)}
      <div class="block">
        <div class="head">
          {#if current?.source !== "todos"}<span class="font-semibold">{t("planPanel.todos")}</span>{/if}
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
  .versions {
    height: 24px;
    max-width: 150px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
    padding: 0 4px;
    font: inherit;
    font-size: 12px;
  }
  .changes-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .changes-toggle input {
    accent-color: var(--accent);
  }
  .diff {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 0;
    overflow-x: auto;
  }
  .dl {
    padding: 0 8px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .dl.add {
    background: var(--add-bg);
  }
  .dl.del {
    color: var(--fg-muted);
    text-decoration: line-through;
    background: var(--del-bg);
  }
</style>
