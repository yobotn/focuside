<script lang="ts">
  // The side panel with Session tabs: Side. Two views, Files and Sessions, picked in the header.
  // Sessions lists the project's open sessions with the same tabs as the identity bar strip.
  import { ArrowsLeftRight, CaretDown, Plus } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app, type Project } from "../stores/app.svelte";
  import FileTree from "./FileTree.svelte";
  import SessionTab from "./SessionTab.svelte";

  let { project }: { project: Project } = $props();

  const view = $derived(app.layout.sideView);
  const waiting = $derived(project.sessions.some((s) => s.waiting));

  function setView(v: "files" | "sessions") {
    app.layout.sideView = v;
    app.save();
  }
</script>

<aside class="side">
  <div class="head">
    <div class="seg" role="tablist">
      <button role="tab" aria-selected={view === "files"} class:on={view === "files"} onclick={() => setView("files")}>
        {t("files.title")}
      </button>
      <button role="tab" aria-selected={view === "sessions"} class:on={view === "sessions"} onclick={() => setView("sessions")}>
        {t("session.list")}
        {#if waiting && view !== "sessions"}<span class="dot pulse-warn" aria-hidden="true"></span>{/if}
      </button>
    </div>
    <span class="flex-1"></span>
    {#if view === "sessions"}
      <button class="icon-btn" title={t("session.new")} aria-label={t("session.new")} onclick={() => app.newSession()}>
        <Plus size={13} />
      </button>
      <button class="icon-btn" title={t("session.newMenu")} aria-label={t("session.newMenu")} onclick={() => (app.overlay = "new")}>
        <CaretDown size={11} />
      </button>
    {/if}
    <button
      class="icon-btn move"
      onclick={() => app.movePanel("files")}
      title={app.layout.filesSide === "left" ? t("layout.moveRight") : t("layout.moveLeft")}
      aria-label={app.layout.filesSide === "left" ? t("layout.moveRight") : t("layout.moveLeft")}
    >
      <ArrowsLeftRight size={12} />
    </button>
  </div>
  <div class="body">
    {#if view === "files"}
      <FileTree {project} head={false} />
    {:else}
      <div class="list" role="tablist" aria-orientation="vertical" aria-label={t("session.list")}>
        {#each project.sessions as x (x.key)}
          <SessionTab session={x} active={x.key === project.activeKey} closable={project.sessions.length > 1} vertical />
        {/each}
        <button class="earlier" onclick={() => (app.overlay = "sessions")}>
          <span class="flex-1">{t("session.earlier")}</span>
          <span class="kbd">{keyLabel("Mod+E")}</span>
        </button>
      </div>
    {/if}
  </div>
</aside>

<style>
  .side {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-subtle);
    min-height: 0;
  }
  .head {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border-radius: 6px;
    background: var(--bg-sunken);
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .seg button:hover {
    color: var(--fg);
  }
  .seg button.on {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 0 0 1px var(--border);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn-line);
  }
  .move {
    opacity: 0;
  }
  .side:hover .move,
  .move:focus-visible {
    opacity: 1;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
  }
  .earlier {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    margin-top: 6px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--fg-muted);
    text-align: start;
  }
  .earlier:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
</style>
