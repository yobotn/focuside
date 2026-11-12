<script lang="ts">
  import { tick } from "svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { reportIssue } from "../updates.svelte";
  import type { Session } from "../stores/session.svelte";
  import ChangesCard from "./ChangesCard.svelte";
  import Markdown from "./Markdown.svelte";
  import PermissionRow from "./PermissionRow.svelte";
  import PlanCard from "./PlanCard.svelte";
  import ToolLine from "./ToolLine.svelte";

  let { session }: { session: Session } = $props();

  let scroller: HTMLDivElement | undefined = $state();
  let stick = true;

  function onscroll() {
    if (!scroller) return;
    stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
  }

  // Follow new output while the user is at the bottom.
  $effect(() => {
    void session.entries.length;
    const last = session.entries[session.entries.length - 1];
    if (last && (last.t === "agent" || last.t === "user")) void last.text.length;
    if (stick) toBottom();
  });

  /** Scrolls to the end after layout settles (off-screen entries size themselves late). */
  function toBottom() {
    tick().then(() => {
      const go = () => scroller?.scrollTo({ top: scroller.scrollHeight });
      go();
      requestAnimationFrame(() => {
        go();
        requestAnimationFrame(go);
      });
    });
  }

  // Switching sessions jumps to the end.
  $effect(() => {
    void session.key;
    stick = true;
    toBottom();
  });

  const lastAgentId = $derived.by(() => {
    for (let i = session.entries.length - 1; i >= 0; i--) if (session.entries[i].t === "agent") return session.entries[i].id;
    return null;
  });

  /** Focus the last card (Alt+Up from the message box). */
  export function focusLastCard() {
    const cards = scroller?.querySelectorAll<HTMLElement>("[data-card]");
    cards?.[cards.length - 1]?.focus();
  }
</script>

<div class="scroller" bind:this={scroller} {onscroll}>
  <div class="column">
    {#if session.entries.length === 0}
      <p class="mt-[20vh] text-center font-read text-[15px] text-fg-muted">{t("conv.empty")}</p>
    {/if}

    {#each session.entries as entry (entry.id)}
      <div class="entry entry-{entry.t}">
        {#if entry.t === "user"}
          <div class="user selectable">
            <p class="whitespace-pre-wrap">{entry.text}</p>
            {#if entry.images || entry.files?.length}
              <p class="mt-1 text-[12px] text-fg-muted">
                {[...(entry.files ?? []).map((f) => "@" + f), entry.images ? `${entry.images} ${t("box.image").toLowerCase()}` : ""].filter(Boolean).join("  ")}
              </p>
            {/if}
          </div>
        {:else if entry.t === "agent"}
          <Markdown text={entry.text} streaming={session.running && entry.id === lastAgentId} />
        {:else if entry.t === "tool"}
          <ToolLine {session} {entry} />
        {:else if entry.t === "permission"}
          <PermissionRow {session} {entry} />
        {:else if entry.t === "plan"}
          <PlanCard {session} {entry} />
        {:else if entry.t === "changes"}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
          <div
            tabindex="0"
            data-card
            role="group"
            aria-label={t("changes.title")}
            class="changes-wrap rounded-lg"
            onkeydown={(e) => {
              if (e.ctrlKey || e.metaKey || e.altKey) return;
              if (e.key.toLowerCase() === "u" && !entry.undo) app.askUndo(session, entry);
              if (e.key.toLowerCase() === "r")
                app.viewer = {
                  kind: "diff",
                  projectId: session.projectId,
                  turnId: entry.turnId,
                  paths: entry.result.changes?.files.map((f) => f.path) ?? [],
                  index: 0,
                  sessionKey: session.key,
                };
            }}
          >
            <ChangesCard {session} result={entry.result} undo={entry.undo} onundo={() => app.askUndo(session, entry)} />
          </div>
        {:else if entry.t === "note"}
          <p
            class="selectable"
            class:text-fg-muted={entry.tone === "muted"}
            class:text-danger={entry.tone === "error"}
            class:text-warn={entry.tone === "warn"}
          >
            {entry.text}
          </p>
        {/if}
      </div>
    {/each}

    {#if session.running && !session.entries.some((e) => e.t === "plan" && (e.state === "planning" || e.state === "running"))}
      <p class="live">{session.thinking ? t("conv.thinking") : t("conv.working")}</p>
    {/if}

    {#if session.status === "stopped" && !session.running}
      <div class="flex items-center gap-3 text-fg-muted">
        <span>{t("session.stopped")}</span>
        <button class="btn" onclick={() => app.resumeAgent(session)}>{t("session.stoppedResume")}</button>
      </div>
    {:else if session.status === "failed"}
      <p class="text-danger selectable">{t("session.failedStart", { agent: app.agentName(session.agent), error: session.error ?? "" })}</p>
      <button
        class="btn self-start"
        onclick={() =>
          reportIssue("bug", { version: app.env?.version, os: app.env?.os, details: `${app.agentName(session.agent)} did not start: ${session.error ?? ""}` })}
      >
        {t("help.reportThis")}
      </button>
    {:else if session.status === "starting" && !session.entries.length}
      <p class="text-center text-fg-muted">{t("session.starting", { agent: app.agentName(session.agent) })}</p>
    {/if}
  </div>
</div>

<style>
  .scroller {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
  }
  .column {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  /* Off-screen entries skip layout and paint: cheap virtualization for long conversations. */
  .entry {
    content-visibility: auto;
    contain-intrinsic-size: auto 28px;
    min-width: 0;
  }
  .entry-tool + .entry-tool {
    margin-top: -10px;
  }
  .entry-user {
    margin-top: 14px;
  }
  .user {
    font-family: var(--font-read);
    font-size: var(--read-size);
    line-height: 1.55;
    font-weight: 600;
    border-inline-start: 2px solid var(--project);
    padding-inline-start: 12px;
  }
  .live {
    color: var(--fg-muted);
  }
  .changes-wrap:focus {
    outline: 2px solid color-mix(in srgb, var(--project) 70%, transparent);
    outline-offset: 2px;
  }
</style>
