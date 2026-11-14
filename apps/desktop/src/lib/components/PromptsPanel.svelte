<script lang="ts">
  // The Prompts panel: every prompt of one session (the active one unless `sessionKey` is
  // given), oldest at the top and newest at the bottom, like a chat. Each row is a dot, the
  // first line of the prompt and the time. A click opens the full prompt, the agent's answer
  // (Markdown) and the files the prompt changed, with Undo. So a long session can be read one
  // prompt at a time.
  //
  // Data: `Session.promptList`. Pane prompts come from hooks (UserPromptSubmit and Stop) and are
  // kept in FocusIDE's app data; chat prompts come from the conversation. No required props, so
  // the dock can place the panel anywhere. `app.showPrompts(key, id)` opens it at one prompt.
  import { tick } from "svelte";
  import { CaretRight } from "phosphor-svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";
  import type { PromptTurn, TurnResult } from "../types";
  import FileIcon from "./FileIcon.svelte";
  import Markdown from "./Markdown.svelte";

  let { sessionKey }: { sessionKey?: string } = $props();

  const session = $derived<Session | undefined>(sessionKey ? app.sessionFor(sessionKey) : app.session);
  const prompts = $derived(session?.promptList ?? []);

  let list: HTMLElement | undefined = $state();
  let open = $state<Record<string, boolean>>({});
  let seenCount = 0;
  let seenFocus = 0;
  let seenSession: string | undefined;

  function firstLine(text: string): string {
    return text.trim().split("\n")[0] || t("prompts.empty");
  }

  function time(at: number | null): string {
    if (!at) return "";
    return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  async function scrollTo(id: string | null, behavior: ScrollBehavior) {
    await tick();
    if (!list) return;
    const el = id ? list.querySelector<HTMLElement>(`[data-prompt="${CSS.escape(id)}"]`) : null;
    if (el) el.scrollIntoView({ block: "nearest", behavior });
    else list.scrollTop = list.scrollHeight;
  }

  // A new prompt scrolls to the bottom, like a chat. A switch of session does too.
  $effect(() => {
    const n = prompts.length;
    const key = session?.key;
    if (key !== seenSession) {
      seenSession = key;
      seenCount = n;
      scrollTo(null, "auto");
    } else if (n > seenCount) {
      seenCount = n;
      scrollTo(null, "smooth");
    }
  });

  // A click on a dot of the pane strip opens the panel at that prompt.
  $effect(() => {
    const f = app.promptsFocus;
    if (!f || f.seq === seenFocus || !session || f.sessionKey !== session.key) return;
    seenFocus = f.seq;
    if (f.id) {
      open[f.id] = true;
      scrollTo(f.id, "smooth");
    }
  });

  function review(s: Session, r: TurnResult, index: number) {
    const files = r.changes?.files ?? [];
    if (!files.length) return;
    app.viewer = { kind: "diff", projectId: s.projectId, turnId: r.turnId, paths: files.map((f) => f.path), index, sessionKey: s.key };
  }

  function stateText(p: PromptTurn): string {
    if (p.untracked != null) return t("prompts.untracked");
    const n = p.result?.changes?.files.length ?? 0;
    return n ? t("planPanel.turn", { count: n }) : "";
  }

  function noAnswer(p: PromptTurn, last: boolean): string {
    return last && session?.busy ? t("prompts.working") : t("prompts.noAnswer");
  }
</script>

<section class="panel" aria-label={t("prompts.title")}>
  {#if !session || !prompts.length}
    <p class="empty text-fg-muted">{t("prompts.none")}</p>
  {:else}
    {@const s = session}
    <ol class="list" bind:this={list}>
      {#each prompts as p, i (p.id)}
        {@const files = p.result?.changes?.files ?? []}
        <li data-prompt={p.id} class:open={open[p.id]}>
          <button class="row" aria-expanded={!!open[p.id]} onclick={() => (open[p.id] = !open[p.id])}>
            <span class="dot" class:changed={files.length > 0} class:untracked={p.untracked != null} aria-hidden="true"></span>
            <span class="caret" class:rot={open[p.id]}><CaretRight size={11} /></span>
            <span class="first truncate">{firstLine(p.prompt)}</span>
            {#if stateText(p)}<span class="meta" class:text-warn={p.untracked != null}>{stateText(p)}</span>{/if}
            {#if p.at}<span class="meta">{time(p.at)}</span>{/if}
          </button>
          {#if open[p.id]}
            <div class="detail">
              <div class="label">{t("prompts.prompt")}</div>
              <p class="prompt selectable">{p.prompt || t("prompts.empty")}</p>
              <div class="label">{t("prompts.answer")}</div>
              {#if p.answer}
                <Markdown class="answer selectable" text={p.answer} />
              {:else}
                <p class="text-fg-muted">{noAnswer(p, i === prompts.length - 1)}</p>
              {/if}
              {#if p.untracked != null}
                <p class="text-warn">{p.untracked ? t("pane.notTracked", { name: p.untracked }) : t("pane.notTrackedOther")}</p>
              {/if}
              {#if files.length && p.result}
                {@const result = p.result}
                <div class="label files-head">
                  <span>{t("planPanel.turn", { count: files.length })}</span>
                  <span class="text-add">+{result.changes?.added ?? 0}</span>
                  <span class="text-del">-{result.changes?.removed ?? 0}</span>
                  <span class="flex-1"></span>
                  {#if p.undo}
                    <span class:text-accent={p.undo.outcome.exact} class:text-warn={!p.undo.outcome.exact}>{t("planPanel.undone")}</span>
                  {:else if p.turnId}
                    <button class="btn h-6" onclick={() => app.askUndo(s, { id: p.id, turnId: p.turnId })}>{t("changes.undo")}</button>
                  {/if}
                </div>
                <ul class="files">
                  {#each files as f, fi (f.path)}
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
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ol>
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
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
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
  .dot.untracked {
    border-color: var(--warn);
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
  :global(.answer) {
    font-size: 13px;
  }
</style>
