<script lang="ts">
  // Pane mode: the agent's own unmodified TUI (for example `claude`) in a terminal that takes
  // the whole area. The user types every prompt there. The plan, the todo list and the changes
  // of each prompt show in the plan panel (PlanPanel.svelte), from hook events. FocusIDE never
  // types into the pane (see stores/paneterm). The one exception is the mode button of a Claude
  // pane: on a click it sends one Shift+Tab key sequence, the key the user would press.
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
  // The mode button needs Claude and hook events: they say when no prompt runs, so Shift+Tab
  // cannot answer a plan or permission question by accident.
  const modeButton = $derived(session.agent === "claude" && (session.hooksExpected || session.hooksLive));
  const planOn = $derived(session.paneMode === "plan");

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
  const otherMode = $derived(planOn ? "" : modeName(session.paneMode));

  const status = $derived.by(() => {
    if (pane?.exited || (!session.paneAlive && session.status === "ready")) return { text: t("pane.exited", { agent: name }), tone: "muted" };
    if (!session.paneAlive) return { text: t("session.starting", { agent: name }), tone: "muted" };
    if (session.tracking === "otherRunning") return { text: t("pane.notTrackedHook"), tone: "warn" };
    if (session.tracking === "error") return { text: t("pane.trackingFailed", { error: session.trackingError ?? "" }), tone: "warn" };
    return null;
  });

  async function switchMode() {
    await app.paneModeKey(session);
    pane?.focus();
  }

  onMount(() => {
    pane = paneTerms.get(session.key, project.id);
    if (session.agent === "claude") {
      const s = session;
      pane.onMode = (m) => (s.paneMode = m);
    }
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
  {#if modeButton || status}
    <div class="bar">
      {#if modeButton}
        <button
          class="btn mode"
          class:on={planOn}
          aria-pressed={planOn}
          disabled={!session.paneAlive || session.paneWorking}
          title={session.paneWorking ? t("pane.modeBusy") : t("pane.modeSwitch")}
          onclick={switchMode}
        >
          <span class="dot" aria-hidden="true"></span>
          {t("pane.planMode")}
        </button>
        {#if otherMode}<span class="truncate">{t("pane.modeNow", { mode: otherMode })}</span>{/if}
      {/if}
      <span class="flex-1"></span>
      {#if status}
        <span class="status truncate" class:text-warn={status.tone === "warn"} title={status.text}>{status.text}</span>
      {/if}
    </div>
  {/if}
  <div class="term" bind:this={host}></div>
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
  .term {
    flex: 1;
    min-width: 0;
    min-height: 120px;
    padding: 6px 0 4px 10px;
    background: var(--bg-sunken);
  }
</style>
