<script lang="ts">
  // Projects on top. Below them, after a line, the side bar views (Files and Sessions): click
  // one to show it, click the one that is shown to hide the side bar. Settings at the bottom.
  import { Files, Gear, Plus, Stack } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";

  type View = "files" | "sessions";
  const views: { id: View; label: string }[] = $derived([
    { id: "files", label: t("files.title") },
    { id: "sessions", label: t("session.list") },
  ]);
  const shownView = $derived(dock.shown("side") ? app.layout.sideView : null);
  const waiting = $derived(!!app.active && app.projectWaiting(app.active));

  function pick(v: View) {
    if (shownView === v) {
      dock.close("side");
      return;
    }
    app.layout.sideView = v;
    dock.open("side");
    app.save();
  }

  function onkey(e: KeyboardEvent, i: number) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const list = (e.currentTarget as HTMLElement).parentElement;
    const next = (i + (e.key === "ArrowDown" ? 1 : -1) + views.length) % views.length;
    list?.querySelectorAll<HTMLElement>("button")[next]?.focus();
  }

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
  {#if app.active}
    <div class="views" role="tablist" aria-orientation="vertical" aria-label={t("side.views")}>
      {#each views as v, i (v.id)}
        <button
          class="view"
          class:active={shownView === v.id}
          role="tab"
          aria-selected={shownView === v.id}
          aria-label={v.label}
          title={shownView === v.id ? t("side.hide", { view: v.label, key: keyLabel("Mod+B") }) : v.label}
          onclick={() => pick(v.id)}
          onkeydown={(e) => onkey(e, i)}
        >
          {#if v.id === "files"}<Files size={18} />{:else}<Stack size={18} />{/if}
          {#if v.id === "sessions" && waiting && shownView !== "sessions"}
            <span class="dot small pulse-warn" aria-hidden="true"></span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
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
  .views {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
  .view {
    position: relative;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--fg-faint);
  }
  .view:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .view.active {
    color: var(--fg);
    background: var(--bg-active);
  }
  .view.active::before {
    content: "";
    position: absolute;
    inset-inline-start: -8px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--fg-muted);
  }
  .dot.small {
    top: 3px;
    inset-inline-end: 3px;
    width: 8px;
    height: 8px;
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
