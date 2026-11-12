<script lang="ts">
  import { GitBranch, Plus, X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app } from "../stores/app.svelte";

  const p = $derived(app.active);
  const s = $derived(app.session);
  let editing = $state(false);
  let draft = $state("");
  let input: HTMLInputElement | undefined = $state();

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
  <header class="bar">
    <span class="project" style="color: hsl({p.hue} var(--project-s) var(--project-l))">{p.name}</span>
    {#if p.branch}
      <span class="item"><GitBranch size={13} />{p.branch}</span>
    {/if}
    <span class="sep">.</span>
    {#if editing}
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
    {#if p.sessions.length > 1}
      <span class="tabs">
        {#each p.sessions as x (x.key)}
          <button
            class="tab"
            class:on={x.key === s?.key}
            title={x.name || t("session.untitled")}
            onclick={() => app.showSession(x.key)}
            aria-label={x.name || t("session.untitled")}
          >
            <span class="pip" class:waiting={!!x.waiting} class:running={x.running}></span>
          </button>
        {/each}
      </span>
    {/if}
    <button class="icon-btn" title={t("session.new")} aria-label={t("session.new")} onclick={() => app.newSession()}>
      <Plus size={14} />
    </button>
    {#if s && p.sessions.length > 1}
      <button class="icon-btn" title={t("common.close")} aria-label={t("common.close")} onclick={() => app.closeSession(s.key)}>
        <X size={13} />
      </button>
    {/if}

    <span class="flex-1"></span>

    {#if s}
      <span class="item agent">Claude Code</span>
      {#if model}
        <button class="item link agent" onclick={() => (app.overlay = "model")} title={t("bar.model")}>{modelName}</button>
      {/if}
      {#if context !== null}
        <span class="item" class:text-warn={context >= 80}>{t("bar.context", { percent: context })}</span>
      {/if}
    {/if}
    {#if app.queue.length}
      <button class="waiting" onclick={() => (app.overlay = "queue")}>
        <span class="dot pulse-warn"></span>
        {t("bar.waiting", { count: app.queue.length })}
        <span class="kbd">{keyLabel("Mod+J")}</span>
      </button>
    {/if}
  </header>
{/if}

<style>
  .bar {
    container-type: inline-size;
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
  @container (max-width: 720px) {
    .agent {
      display: none;
    }
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
  .tabs {
    display: inline-flex;
    gap: 2px;
  }
  .tab {
    display: grid;
    place-items: center;
    width: 16px;
    height: 20px;
    border: 0;
    background: transparent;
    border-radius: 4px;
  }
  .tab:hover {
    background: var(--bg-hover);
  }
  .pip {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--border-strong);
  }
  .tab.on .pip {
    background: var(--fg);
  }
  .pip.running {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--fg) 20%, transparent);
  }
  .pip.waiting {
    background: var(--warn-line);
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
