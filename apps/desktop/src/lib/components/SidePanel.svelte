<script lang="ts">
  // The side bar: the file tree or the project's open sessions. The view is picked with the
  // Files and Sessions icons on the rail. Sessions shows the same tabs as the identity bar
  // strip; drag a tab to change the order. Drag the header to move the side bar (DockGrip).
  import { CaretDown, Plus } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { reorder } from "../reorder";
  import { app, type Project } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";
  import DockGrip from "./DockGrip.svelte";
  import FileTree from "./FileTree.svelte";
  import SessionTab from "./SessionTab.svelte";

  let { project }: { project: Project } = $props();

  const view = $derived(app.layout.sideView);
</script>

<aside class="side" aria-label={view === "files" ? t("files.title") : t("session.list")}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="head" onpointerdown={(e) => dock.fromHeader("side", e)}>
    <span class="title">{view === "files" ? t("files.title") : t("session.list")}</span>
    <span class="flex-1"></span>
    {#if view === "sessions"}
      <button class="icon-btn" title={t("session.new")} aria-label={t("session.new")} onclick={() => app.newSession()}>
        <Plus size={13} />
      </button>
      <button class="icon-btn" title={t("session.newMenu")} aria-label={t("session.newMenu")} onclick={() => (app.overlay = "new")}>
        <CaretDown size={11} />
      </button>
    {/if}
    <DockGrip panel="side" />
  </div>
  <div class="body">
    {#if view === "files"}
      <FileTree {project} />
    {:else}
      <div class="list" role="tablist" aria-orientation="vertical" aria-label={t("session.list")} {@attach reorder("y", (key, i) => app.moveSession(key, i))}>
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
    padding: 0 4px 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
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
