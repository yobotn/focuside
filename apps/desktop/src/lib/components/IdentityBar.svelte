<script lang="ts">
  import { CaretDown, GitBranch, Plus, X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app } from "../stores/app.svelte";
  import { updates } from "../updates.svelte";
  import { reorder } from "../reorder";
  import SessionTab from "./SessionTab.svelte";
  import DockGrip from "./DockGrip.svelte";
  import { dock } from "../stores/dock.svelte";

  function restart() {
    if (app.projects.some((pr) => pr.sessions.some((x) => x.running || x.paneAlive))) {
      app.flash(t("update.busy"), "error");
      return;
    }
    updates.installAndRestart();
  }

  const p = $derived(app.active);
  const s = $derived(app.session);
  let editing = $state(false);
  let draft = $state("");
  let input: HTMLInputElement | undefined = $state();
  let width = $state(1000);
  let strip: HTMLDivElement | undefined = $state();
  // Session tabs in the top bar: on shows every session here. The side bar always lists them too.
  const top = $derived(app.settings.sessionTabs !== "side");

  // The active tab stays in view when the strip scrolls.
  $effect(() => {
    const key = s?.key;
    if (!strip || !key) return;
    strip.querySelector(`[data-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  // A mouse wheel scrolls the crowded strip sideways.
  $effect(() => {
    const el = strip;
    if (!el) return;
    const onwheel = (e: WheelEvent) => {
      if (!e.deltaY || e.deltaX || el.scrollWidth <= el.clientWidth) return;
      el.scrollLeft += e.deltaY;
      e.preventDefault();
    };
    el.addEventListener("wheel", onwheel, { passive: false });
    return () => el.removeEventListener("wheel", onwheel);
  });

  const context = $derived(s?.usage && s.usage.size ? Math.round((s.usage.used / s.usage.size) * 100) : null);
  const model = $derived(s?.model);
  const modelName = $derived(model?.options.find((o) => o.value === model.currentValue)?.name ?? model?.currentValue ?? "");

  function startRename() {
    if (!s) return;
    draft = s.name;
    editing = true;
    queueMicrotask(() => input?.select());
  }
  function commit() {
    if (s && editing) app.renameSession(s, draft);
    editing = false;
  }
</script>

{#if p}
  <!-- The header of the Session panel: drag it (not the session tabs) or the grip to move the panel. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header
    class="bar"
    class:narrow={width < 640}
    bind:clientWidth={width}
    onpointerdown={(e) => {
      if (!(e.target as Element).closest("[role=tablist]")) dock.fromHeader("session", e);
    }}
  >
    <span class="project" style="color: hsl({p.hue} var(--project-s) var(--project-l))">{p.name}</span>
    {#if p.branch}
      <span class="item"><GitBranch size={13} />{p.branch}</span>
    {/if}
    <span class="sep">.</span>
    {#if top}
      <div class="strip" role="tablist" aria-label={t("session.list")} bind:this={strip} {@attach reorder("x", (key, i) => app.moveSession(key, i))}>
        {#each p.sessions as x (x.key)}
          <SessionTab session={x} active={x.key === s?.key} closable={p.sessions.length > 1} />
        {/each}
      </div>
    {:else if editing}
      <input
        bind:this={input}
        bind:value={draft}
        class="rename"
        onblur={commit}
        onkeydown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") editing = false;
        }}
      />
    {:else}
      <button class="session truncate" ondblclick={startRename} onclick={() => (app.overlay = "sessions")} title={t("session.renameHint")}>
        {s?.name || t("session.untitled")}
      </button>
    {/if}
    <button class="icon-btn" title={t("session.new")} aria-label={t("session.new")} onclick={() => app.newSession()}>
      <Plus size={14} />
    </button>
    <button class="icon-btn" title={t("session.newMenu")} aria-label={t("session.newMenu")} onclick={() => (app.overlay = "new")}>
      <CaretDown size={12} />
    </button>
    {#if !top && s && p.sessions.length > 1}
      <button class="icon-btn" title={t("session.close")} aria-label={t("session.close")} onclick={() => app.closeSession(s.key)}>
        <X size={13} />
      </button>
    {/if}

    <span class="flex-1"></span>

    {#if s}
      <span class="item agent">{app.agentName(s.agent)}</span>
      {#if model}
        <button class="item link agent" onclick={() => (app.overlay = "model")} title={t("bar.model")}>{modelName}</button>
      {/if}
      {#if context !== null}
        <span class="item" class:text-warn={context >= 80}>{t("bar.context", { percent: context })}</span>
      {/if}
    {/if}
    {#if updates.state.kind === "ready"}
      <button class="update" onclick={restart} title={t("update.readyHint", { version: updates.state.version })}>
        {t("update.ready", { version: updates.state.version })}
      </button>
    {:else if updates.state.kind === "installing"}
      <span class="item">{t("update.installing", { version: updates.state.version })}</span>
    {/if}
    {#if app.queue.length}
      <button class="waiting" onclick={() => (app.overlay = "queue")}>
        <span class="dot pulse-warn"></span>
        {t("bar.waiting", { count: app.queue.length })}
        <span class="kbd">{keyLabel("Mod+J")}</span>
      </button>
    {/if}
    <DockGrip panel="session" />
  </header>
{/if}

<style>
  .update {
    flex-shrink: 0;
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    border: 1px solid var(--accent);
    background: var(--accent-soft);
    color: var(--fg);
    font-size: 12px;
    white-space: nowrap;
  }
  .update:hover {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 12px 0 16px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
    min-width: 0;
  }
  .project {
    font-weight: 700;
    flex-shrink: 0;
  }
  .session {
    min-width: 48px;
  }
  .narrow .agent {
    display: none;
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--fg-muted);
  }
  .link {
    background: none;
    border: 0;
    padding: 2px 4px;
    border-radius: 4px;
  }
  .link:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .sep {
    color: var(--fg-faint);
  }
  .session {
    background: none;
    border: 0;
    padding: 2px 4px;
    border-radius: 4px;
    max-width: 320px;
    font-weight: 500;
  }
  .session:hover {
    background: var(--bg-hover);
  }
  .rename {
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    padding: 1px 4px;
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    width: 260px;
    user-select: text;
  }
  .strip {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    flex: 0 1 auto;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
  }
  .waiting {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border-radius: 6px;
    border: 1px solid color-mix(in srgb, var(--warn-line) 50%, var(--border));
    background: var(--warn-soft);
    color: var(--warn);
    font-weight: 600;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warn-line);
  }
</style>
