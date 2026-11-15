<script lang="ts">
  // The Prompts panel: every prompt of one session (the active one unless `sessionKey` is
  // given), oldest at the top and newest at the bottom, like a chat. Two views (Settings keep
  // the choice, `settings.promptsView`):
  // - List: one row per prompt with a dot, the first line, the number of changed files and the
  //   time. A click opens the full prompt, the answer (Markdown) and the changed files with Undo.
  // - Conversation: only the prompts and answers, as one document to read. No tool output.
  // Both views step from prompt to prompt (Previous and Next, Alt+Up and Alt+Down, Home and
  // End), filter by a search, copy a prompt or an answer, and undo back to a prompt.
  //
  // Data: `Session.promptList`. Pane prompts come from hooks (UserPromptSubmit and Stop) and are
  // kept in FocusIDE's app data; chat prompts come from the conversation. No required props, so
  // the dock can place the panel anywhere. `app.showPrompts(key, id)` opens it at one prompt.
  // The current prompt per session is `app.promptsCurrent`; the dot strip of the pane shows it.
  import { tick } from "svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { CaretDown, CaretRight, CaretUp, Copy, MagnifyingGlass } from "phosphor-svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";
  import type { PromptTurn, TurnResult } from "../types";
  import { fileCount, filterPrompts, firstLine, promptState, splitMatches, stepPrompt, undoBackFiles, undoBackTurns } from "../prompts";
  import FileIcon from "./FileIcon.svelte";
  import Markdown from "./Markdown.svelte";

  let { sessionKey }: { sessionKey?: string } = $props();

  const session = $derived<Session | undefined>(sessionKey ? app.sessionFor(sessionKey) : app.session);
  const prompts = $derived(session?.promptList ?? []);
  const view = $derived(app.settings.promptsView);

  let list: HTMLElement | undefined = $state();
  let searchBox: HTMLInputElement | undefined = $state();
  let open = $state<Record<string, boolean>>({});
  let query = $state("");
  /** Conversation view: show only the current prompt. */
  let one = $state(false);
  let seenCount = 0;
  let seenFocus = 0;
  let seenSearch = app.promptsSearchSeq;
  let seenSession: string | undefined;

  const visible = $derived(filterPrompts(prompts, query));
  const currentId = $derived(session ? (app.promptsCurrent[session.key] ?? null) : null);
  const currentIndex = $derived(currentId ? prompts.findIndex((p) => p.id === currentId) : -1);
  /** What the view shows: the search result, or only the current prompt in "One at a time". */
  const shown = $derived.by(() => {
    if (view === "conversation" && one) {
      const i = visible.includes(currentIndex) ? currentIndex : visible[visible.length - 1];
      return i === undefined ? [] : [i];
    }
    return visible;
  });

  function lineOf(text: string): string {
    return firstLine(text, 200) || t("prompts.empty");
  }

  function time(at: number | null): string {
    if (!at) return "";
    return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  async function scrollTo(id: string | null, behavior: ScrollBehavior) {
    await tick();
    if (!list) return;
    const el = id ? list.querySelector<HTMLElement>(`[data-prompt="${CSS.escape(id)}"]`) : null;
    if (el) el.scrollIntoView({ block: view === "conversation" ? "start" : "nearest", behavior });
    else list.scrollTop = list.scrollHeight;
  }

  function setCurrent(id: string) {
    if (session) app.promptsCurrent[session.key] = id;
  }

  // A new prompt scrolls to the bottom, like a chat. A switch of session does too.
  $effect(() => {
    const n = prompts.length;
    const key = session?.key;
    if (key !== seenSession) {
      seenSession = key;
      seenCount = n;
      scrollTo(currentId, "auto");
    } else if (n > seenCount) {
      seenCount = n;
      if (!one) scrollTo(null, "smooth");
    }
  });

  // A click on a dot of the pane strip opens the panel at that prompt.
  $effect(() => {
    const f = app.promptsFocus;
    if (!f || f.seq === seenFocus || !session || f.sessionKey !== session.key) return;
    seenFocus = f.seq;
    if (f.id) {
      setCurrent(f.id);
      if (!visible.includes(prompts.findIndex((p) => p.id === f.id))) query = "";
      open[f.id] = true;
      scrollTo(f.id, "smooth");
    }
  });

  // Palette "Search prompts": the focus goes to the search box (after the dock moved it).
  $effect(() => {
    const seq = app.promptsSearchSeq;
    if (seq === seenSearch) return;
    seenSearch = seq;
    setTimeout(() => {
      searchBox?.focus();
      searchBox?.select();
    }, 0);
  });

  function go(move: "prev" | "next" | "first" | "last") {
    const i = stepPrompt(visible, currentIndex, move);
    if (i < 0) return;
    const id = prompts[i].id;
    setCurrent(id);
    if (view === "list") open[id] = true;
    scrollTo(id, "smooth");
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.shiftKey) return;
    const target = e.target as HTMLElement | null;
    const typing = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
    let move: "prev" | "next" | "first" | "last" | null = null;
    if (e.altKey && e.key === "ArrowUp") move = "prev";
    else if (e.altKey && e.key === "ArrowDown") move = "next";
    else if (!e.altKey && !typing && e.key === "Home") move = "first";
    else if (!e.altKey && !typing && e.key === "End") move = "last";
    if (!move) return;
    e.preventDefault();
    go(move);
  }

  function review(s: Session, r: TurnResult, index: number) {
    const files = r.changes?.files ?? [];
    if (!files.length) return;
    app.viewer = { kind: "diff", projectId: s.projectId, turnId: r.turnId, paths: files.map((f) => f.path), index, sessionKey: s.key };
  }

  function stateText(p: PromptTurn, i: number): string {
    if (p.untracked != null) return t("prompts.untracked");
    if (promptState(p, i === prompts.length - 1, !!session?.busy) === "failed") return t("prompts.failed");
    const n = fileCount(p);
    return n ? t("planPanel.turn", { count: n }) : "";
  }

  function noAnswer(i: number): string {
    return i === prompts.length - 1 && session?.busy ? t("prompts.working") : t("prompts.noAnswer");
  }

  async function copy(text: string) {
    try {
      await writeText(text);
    } catch {
      await navigator.clipboard?.writeText(text).catch(() => {});
    }
    app.flash(t("prompts.copied"));
  }

  function undoBack(s: Session, i: number) {
    const turns = undoBackTurns(prompts, i);
    app.askUndoBack(s, prompts[i].id, turns, undoBackFiles(turns));
  }
</script>

{#snippet marked(text: string)}
  {#each splitMatches(text, query) as part, k (k)}{#if part.hit}<mark class="hit">{part.text}</mark>{:else}{part.text}{/if}{/each}
{/snippet}

{#snippet files(s: Session, p: PromptTurn)}
  {@const result = p.result!}
  {@const changed = result.changes?.files ?? []}
  <ul class="files">
    {#each changed as f, fi (f.path)}
      <li>
        <button class="file" title={f.path} onclick={() => review(s, result, fi)}>
          <FileIcon path={f.path} />
          <span class="truncate">{f.path}</span>
          {#if f.added}<span class="text-add">+{f.added}</span>{/if}
          {#if f.removed}<span class="text-del">-{f.removed}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet actions(s: Session, p: PromptTurn, i: number)}
  {@const later = undoBackTurns(prompts, i).length}
  <button class="btn h-6" title={t("prompts.copyPrompt")} onclick={() => copy(p.prompt)}>
    <Copy size={12} />{t("prompts.copyPrompt")}
  </button>
  {#if p.answer}
    <button class="btn h-6" title={t("prompts.copyAnswer")} onclick={() => copy(p.answer ?? "")}>
      <Copy size={12} />{t("prompts.copyAnswer")}
    </button>
  {/if}
  {#if later}
    <button class="btn h-6" title={t("prompts.undoBackTip")} disabled={s.busy} onclick={() => undoBack(s, i)}>{t("prompts.undoBack")}</button>
  {/if}
{/snippet}

{#snippet undoTurn(s: Session, p: PromptTurn)}
  {#if p.undo}
    <span class:text-accent={p.undo.outcome.exact} class:text-warn={!p.undo.outcome.exact}>{t("planPanel.undone")}</span>
  {:else if p.turnId}
    <button class="btn h-6" onclick={() => app.askUndo(s, { id: p.id, turnId: p.turnId })}>{t("changes.undo")}</button>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="panel" aria-label={t("prompts.title")} {onkeydown}>
  {#if !session || !prompts.length}
    <p class="empty text-fg-muted">{t("prompts.none")}</p>
  {:else}
    {@const s = session}
    <div class="bar">
      <div class="seg" role="group" aria-label={t("prompts.view")}>
        <button class="btn h-6" class:on={view === "list"} aria-pressed={view === "list"} onclick={() => app.setPromptsView("list")}>{t("prompts.viewList")}</button>
        <button class="btn h-6" class:on={view === "conversation"} aria-pressed={view === "conversation"} onclick={() => app.setPromptsView("conversation")}>
          {t("prompts.viewConversation")}
        </button>
      </div>
      {#if view === "conversation"}
        <button class="btn h-6" class:on={one} aria-pressed={one} title={t("prompts.oneTip")} onclick={() => (one = !one)}>{t("prompts.one")}</button>
      {/if}
      <span class="flex-1"></span>
      <div class="nav" title={t("prompts.keys")}>
        <button class="btn h-6 icon" aria-label={t("prompts.prev")} title={t("prompts.prev")} onclick={() => go("prev")}><CaretUp size={13} /></button>
        <span class="pos" aria-live="polite">
          {currentIndex >= 0 ? t("prompts.position", { n: currentIndex + 1, count: prompts.length }) : t("prompts.noPosition", { count: prompts.length })}
        </span>
        <button class="btn h-6 icon" aria-label={t("prompts.next")} title={t("prompts.next")} onclick={() => go("next")}><CaretDown size={13} /></button>
      </div>
    </div>
    <div class="search">
      <MagnifyingGlass size={13} />
      <input bind:this={searchBox} type="search" bind:value={query} placeholder={t("prompts.search")} aria-label={t("prompts.search")} spellcheck="false" />
      {#if query.trim()}<span class="meta">{t("prompts.matches", { count: visible.length })}</span>{/if}
    </div>

    {#if !shown.length}
      <p class="empty text-fg-muted">{t("prompts.noMatch")}</p>
    {:else if view === "list"}
      <ol class="list" bind:this={list}>
        {#each shown as i (prompts[i].id)}
          {@const p = prompts[i]}
          {@const n = fileCount(p)}
          {@const state = promptState(p, i === prompts.length - 1, s.busy)}
          <li data-prompt={p.id} class:open={open[p.id]} class:current={p.id === currentId}>
            <button
              class="row"
              aria-expanded={!!open[p.id]}
              data-dock-focus={p.id === currentId ? "" : undefined}
              onclick={() => {
                open[p.id] = !open[p.id];
                setCurrent(p.id);
              }}
            >
              <span class="dot {state}" aria-hidden="true"></span>
              <span class="caret" class:rot={open[p.id]}><CaretRight size={11} /></span>
              <span class="first truncate">{@render marked(lineOf(p.prompt))}</span>
              {#if stateText(p, i)}<span class="meta" class:text-warn={p.untracked != null} class:text-danger={state === "failed"}>{stateText(p, i)}</span>{/if}
              {#if p.at}<span class="meta">{time(p.at)}</span>{/if}
            </button>
            {#if open[p.id]}
              <div class="detail">
                <div class="label">{t("prompts.prompt")}</div>
                <p class="prompt selectable">{@render marked(p.prompt || t("prompts.empty"))}</p>
                <div class="label">{t("prompts.answer")}</div>
                {#if p.answer}
                  <Markdown class="answer selectable" text={p.answer} mark={query} />
                {:else}
                  <p class="text-fg-muted">{noAnswer(i)}</p>
                {/if}
                {#if p.untracked != null}
                  <p class="text-warn">{p.untracked ? t("pane.notTracked", { name: p.untracked }) : t("pane.notTrackedOther")}</p>
                {/if}
                {#if n && p.result}
                  <div class="label files-head">
                    <span>{t("planPanel.turn", { count: n })}</span>
                    <span class="text-add">+{p.result.changes?.added ?? 0}</span>
                    <span class="text-del">-{p.result.changes?.removed ?? 0}</span>
                    <span class="flex-1"></span>
                    {@render undoTurn(s, p)}
                  </div>
                  {@render files(s, p)}
                {/if}
                <div class="acts">{@render actions(s, p, i)}</div>
              </div>
            {/if}
          </li>
        {/each}
      </ol>
    {:else}
      <div class="doc-view list" bind:this={list}>
        {#each shown as i (prompts[i].id)}
          {@const p = prompts[i]}
          {@const n = fileCount(p)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
          <article data-prompt={p.id} class:current={p.id === currentId} onclick={() => setCurrent(p.id)}>
            <blockquote class="q selectable" tabindex="-1" data-dock-focus={p.id === currentId ? "" : undefined}><span class="qn">{i + 1}</span>{@render marked(p.prompt || t("prompts.empty"))}</blockquote>
            {#if p.answer}
              <Markdown class="answer selectable" text={p.answer} mark={query} />
            {:else}
              <p class="text-fg-muted">{noAnswer(i)}</p>
            {/if}
            {#if p.untracked != null}
              <p class="text-warn">{p.untracked ? t("pane.notTracked", { name: p.untracked }) : t("pane.notTrackedOther")}</p>
            {/if}
            <div class="acts">
              {#if p.at}<span class="meta">{time(p.at)}</span>{/if}
              {#if n && p.result}
                <button class="link" aria-expanded={!!open[p.id]} onclick={() => (open[p.id] = !open[p.id])}>{t("planPanel.turn", { count: n })}</button>
                {@render undoTurn(s, p)}
              {/if}
              <span class="flex-1"></span>
              {@render actions(s, p, i)}
            </div>
            {#if n && p.result && open[p.id]}{@render files(s, p)}{/if}
          </article>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    font-size: 13px;
  }
  .empty {
    padding: 12px 14px;
  }
  .bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 8px 0;
  }
  .seg {
    display: inline-flex;
    gap: 2px;
  }
  .on {
    background: var(--bg-active);
    border-color: var(--fg-faint);
    color: var(--fg);
  }
  .nav {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .icon {
    padding: 0 4px;
  }
  .pos {
    font-size: 12px;
    color: var(--fg-muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 8px 0;
    padding: 0 8px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--fg-muted);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--fg);
    font-size: 12.5px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li.current > .row {
    background: var(--bg-active);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-width: 0;
    height: var(--row);
    padding: 0 6px;
    background: transparent;
    border: 0;
    border-radius: 4px;
    text-align: start;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .dot {
    flex: 0 0 auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1px solid var(--fg-muted);
  }
  .dot.changed {
    background: var(--accent);
    border-color: var(--accent);
  }
  .dot.working {
    border-color: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .dot.failed {
    background: var(--danger);
    border-color: var(--danger);
  }
  .dot.undone {
    border-color: var(--accent);
    border-style: dashed;
  }
  .dot.untracked {
    border-color: var(--warn);
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.working {
      animation: none;
    }
  }
  .caret {
    display: inline-flex;
    color: var(--fg-muted);
    transition: transform 120ms;
  }
  .caret.rot {
    transform: rotate(90deg);
  }
  .first {
    flex: 1;
    min-width: 0;
  }
  .meta {
    flex: 0 0 auto;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 6px 10px 28px;
    min-width: 0;
  }
  .label {
    margin-top: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-muted);
  }
  .files-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .prompt {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
    font-size: 12px;
  }
  .acts :global(.btn) {
    gap: 4px;
  }
  .files {
    padding: 2px 0 0;
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
  /* Conversation view: one readable document. */
  .doc-view {
    gap: 0;
    padding: 4px 14px 24px;
  }
  .doc-view article {
    padding: 14px 10px 12px;
    border-bottom: 1px solid var(--border);
    border-inline-start: 2px solid transparent;
    min-width: 0;
  }
  .doc-view article.current {
    border-inline-start-color: var(--accent);
    background: var(--bg-hover);
  }
  .q {
    margin: 0 0 10px;
    padding: 6px 10px;
    border-inline-start: 3px solid var(--fg-faint);
    background: var(--bg-sunken);
    border-radius: 0 5px 5px 0;
    font-weight: 600;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    outline: none;
  }
  .qn {
    margin-inline-end: 6px;
    font-weight: 400;
    font-size: 12px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
  .link {
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--accent);
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  mark.hit,
  :global(mark.hit) {
    background: color-mix(in srgb, var(--warn) 35%, transparent);
    color: inherit;
    border-radius: 2px;
  }
  :global(.answer) {
    font-size: 13px;
  }
</style>
