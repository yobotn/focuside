<script lang="ts">
  import { Plus, Gear } from "phosphor-svelte";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";

  function color(hue: number) {
    return `hsl(${hue} var(--project-s) var(--project-l))`;
  }
</script>

<nav class="rail" aria-label="Projects">
  {#each app.projects as p (p.id)}
    <button
      class="mono"
      class:active={p.id === app.activeId}
      style="--c: {color(p.hue)}"
      title={p.root}
      onclick={() => app.activate(p.id)}
      onauxclick={(e) => e.button === 1 && app.closeProject(p.id)}
    >
      {p.monogram}
      {#if app.projectWaiting(p)}
        <span class="dot pulse-warn" title={t("rail.waiting")}></span>
      {/if}
    </button>
  {/each}
  <button class="add" title={t("rail.open")} aria-label={t("rail.open")} onclick={() => app.pickFolder()}>
    <Plus size={16} />
  </button>
  <span class="flex-1"></span>
  <button class="add" title={t("cmd.settings")} aria-label={t("cmd.settings")} onclick={() => (app.overlay = "settings")}>
    <Gear size={16} />
  </button>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 48px;
    padding: 10px 0;
    background: var(--bg-sunken);
    border-inline-end: 1px solid var(--border);
    overflow-y: auto;
  }
  .mono {
    position: relative;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: 8px;
    border: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  .mono:hover {
    background: color-mix(in srgb, var(--c) 24%, transparent);
  }
  .mono.active {
    background: var(--c);
    color: var(--bg);
  }
  .mono.active::before {
    content: "";
    position: absolute;
    inset-inline-start: -8px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--c);
  }
  .dot {
    position: absolute;
    top: -3px;
    inset-inline-end: -3px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--warn-line);
    border: 2px solid var(--bg-sunken);
  }
  .add {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: 8px;
    border: 1px dashed var(--border-strong);
    background: transparent;
    color: var(--fg-muted);
  }
  .add:last-child {
    border: 0;
  }
  .add:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
</style>
