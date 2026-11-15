<script lang="ts">
  // Pane mode: the agent's own unmodified TUI (for example `claude`) in a terminal that takes
  // the whole area. The user types every prompt there. The plan, the todo list and the changes
  // of each prompt show in the plan panel (PlanPanel.svelte), and each prompt with its answer in
  // the Prompts panel (PromptsPanel.svelte), from hook events. FocusIDE never types prompts
  // into the pane (see stores/paneterm). The plan mode switch of a Claude pane sends Shift+Tab
  // key sequences on a click, the key the user would press.
  import { onDestroy, onMount } from "svelte";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import { paneTerms, type PaneTerm } from "../stores/paneterm.svelte";
  import type { Session } from "../stores/session.svelte";

  let { session, project }: { session: Session; project: Project } = $props();

  let host: HTMLDivElement | undefined = $state();
  let pane: PaneTerm | null = $state(null);
  let ro: ResizeObserver | null = null;

  const name = $derived(app.agentName(session.agent));
  // The mode switch needs Claude and hook events: they say when no prompt runs, so Shift+Tab
  // cannot answer a plan or permission question by accident.
  const modeButton = $derived(session.agent === "claude" && (session.hooksExpected || session.hooksLive));
  // The honest state: what Claude's status line shows now. Null: it cannot be read.
  const shown = $derived(session.screenMode);
  const planOn = $derived(shown === "plan");
  const modeReady = $derived(session.paneAlive && !session.paneWorking && !!shown && !session.modeSwitching);
  const modeTip = $derived(
    session.paneWorking
      ? t("pane.modeBusy")
      : !shown
        ? t("pane.modeUnknown")
        : planOn
          ? t("pane.planModeOff")
          : t("pane.planModeOn"),
  );

  function modeName(m: string | null): string {
    switch (m) {
      case "default":
        return t("pane.mode.default");
      case "acceptEdits":
        return t("pane.mode.acceptEdits");
      case "auto":
        return t("pane.mode.auto");
      case "bypassPermissions":
        return t("pane.mode.bypassPermissions");
      case "dontAsk":
        return t("pane.mode.dontAsk");
      default:
        return "";
    }
  }
  const otherMode = $derived(planOn ? "" : modeName(shown));

  const status = $derived.by(() => {
    if (pane?.exited || (!session.paneAlive && session.status === "ready")) return { text: t("pane.exited", { agent: name }), tone: "muted" };
    if (!session.paneAlive) return { text: t("session.starting", { agent: name }), tone: "muted" };
    // Shown until the first prompt in the new conversation.
    if (session.resumeFailed) return { text: t("pane.resumeFailed", { agent: name }), tone: "warn" };
    if (session.tracking === "otherRunning")
      return {
        text: session.untrackedBy ? t("pane.notTracked", { name: session.untrackedBy }) : t("pane.notTrackedOther"),
        tone: "warn",
      };
    if (session.tracking === "error") return { text: t("pane.trackingFailed", { error: session.trackingError ?? "" }), tone: "warn" };
    return null;
  });

  const prompts = $derived(session.promptList);

  function firstLine(text: string): string {
    const line = text.trim().split("\n")[0] ?? "";
    return line.length > 80 ? line.slice(0, 77) + "..." : line || t("prompts.empty");
  }

  onMount(() => {
    pane = paneTerms.get(session.key, project.id, session.agent);
    const s = session;
    if (s.agent === "claude") {
      pane.onMode = (m) => {
        s.screenMode = m;
        if (m) s.paneMode = m;
      };
      s.screenMode = pane.shownMode();
    }
    // Any agent that sets a terminal title names the tab (see tui.ts, titleTopic).
    pane.onTitle = (title) => app.paneTitle(s, title);
    pane.attach(host!);
    ro = new ResizeObserver(() => pane?.resize());
    ro.observe(host!);
  });

  onDestroy(() => {
    ro?.disconnect();
    // The TUI keeps running while another session is shown.
    pane?.detach();
  });
</script>

<div class="view">
  <div class="bar">
    {#if modeButton}
      <button
        class="btn mode"
        class:on={planOn}
        aria-pressed={planOn}
        disabled={!modeReady}
        title={modeTip}
        onclick={() => app.togglePlanMode(session)}
      >
        <span class="dot" aria-hidden="true"></span>
        {t("pane.planMode")}
      </button>
      {#if otherMode}<span class="truncate">{t("pane.modeNow", { mode: otherMode })}</span>{/if}
    {/if}
    <button class="btn" title={t("pane.showPlan")} onclick={() => app.showPlan()}>{t("planPanel.title")}</button>
    <span class="flex-1"></span>
    {#if status}
      <span class="status truncate" class:text-warn={status.tone === "warn"} title={status.text}>{status.text}</span>
    {/if}
  </div>
  <div class="body">
    <div class="term" bind:this={host}></div>
    {#if prompts.length}
      <nav class="dots" aria-label={t("prompts.title")}>
        {#each prompts as p, i (p.id)}
          <button
            class="pdot"
            class:untracked={p.untracked != null}
            class:changed={!!p.result?.changes?.files.length}
            title={firstLine(p.prompt)}
            aria-label={t("prompts.dot", { n: i + 1, text: firstLine(p.prompt) })}
            onclick={() => app.showPrompts(session.key, p.id)}
          ></button>
        {/each}
      </nav>
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
  .status {
    min-width: 0;
    flex: 0 1 auto;
  }
  .mode {
    gap: 6px;
  }
  .mode .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1px solid var(--fg-muted);
  }
  .mode.on {
    color: var(--fg);
    border-color: var(--accent);
  }
  .mode.on .dot {
    background: var(--accent);
    border-color: var(--accent);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .term {
    flex: 1;
    min-width: 0;
    min-height: 120px;
    padding: 6px 0 4px 10px;
    background: var(--bg-sunken);
  }
  /* One dot per prompt along the pane edge. A click opens the Prompts panel at that prompt. */
  .dots {
    flex: 0 0 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    overflow-y: auto;
    scrollbar-width: none;
    background: var(--bg-sunken);
    border-left: 1px solid var(--border);
  }
  .pdot {
    flex: 0 0 auto;
    width: 8px;
    height: 8px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--fg-muted);
    background: transparent;
  }
  .pdot.changed {
    background: var(--accent);
    border-color: var(--accent);
  }
  .pdot.untracked {
    border-color: var(--warn);
  }
  .pdot:hover,
  .pdot:focus-visible {
    transform: scale(1.4);
  }
</style>
