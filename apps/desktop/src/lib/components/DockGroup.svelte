<script lang="ts">
  // One dock group: its visible panels as tabs, and a slot for each panel. The panels render
  // once in App and move into their slot (dockHost), so moving a group or a tab does not restart
  // the terminal or reload the viewer. The tab strip shows with two or more visible panels, or
  // for a panel without its own header (the plan).
  import { ChatText, ListChecks, MagnifyingGlass, X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { shownPanel, type DockGroup, type PanelId, type Zone } from "../dock";
  import { dock, dockSlot } from "../stores/dock.svelte";
  import { app } from "../stores/app.svelte";
  import DockGrip from "./DockGrip.svelte";
  import { panelName } from "./dockNames";

  let { zone, group }: { zone: Zone; group: DockGroup } = $props();

  const OWN_HEADER = new Set<PanelId>(["session", "side", "search", "viewer", "terminal"]);

  const visible = $derived(group.panels.filter((p) => dock.visible(p)));
  const shown = $derived(shownPanel(group, (p) => dock.visible(p)));
  const strip = $derived(visible.length > 1 || (!!shown && !OWN_HEADER.has(shown)));

  function pick(p: PanelId) {
    group.active = p;
    app.save();
  }

  function ontabkey(e: KeyboardEvent, i: number) {
    const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const list = (e.currentTarget as HTMLElement).closest("[role=tablist]");
    const next = visible[(i + step + visible.length) % visible.length];
    pick(next);
    queueMicrotask(() => list?.querySelector<HTMLElement>(`[data-tab="${next}"]`)?.focus());
  }
</script>

<div class="group" data-dock-group={group.id} data-dock-zone={zone}>
  {#if strip}
    <div class="strip">
      <div class="tabs" role="tablist" aria-label={t("dock.panels")}>
        {#each visible as p, i (p)}
          <div class="tab" class:on={p === shown} class:fixed={p === "session"}>
            <button
              role="tab"
              data-tab={p}
              aria-selected={p === shown}
              tabindex={p === shown ? 0 : -1}
              title={t("dock.tabHint")}
              onpointerdown={(e) => dock.start(p, e)}
              onclick={() => pick(p)}
              onkeydown={(e) => ontabkey(e, i)}
            >
              {#if p === "plan"}<ListChecks size={13} />{:else if p === "prompts"}<ChatText size={13} />{:else if p === "search"}<MagnifyingGlass size={13} />{/if}
              {panelName(p)}
            </button>
            {#if p !== "session"}
              <button class="close" title={t("common.close")} aria-label={t("dock.close", { panel: panelName(p) })} onclick={() => dock.close(p)}>
                <X size={11} />
              </button>
            {/if}
          </div>
        {/each}
      </div>
      <span class="flex-1"></span>
      {#if shown}<DockGrip panel={shown} />{/if}
    </div>
  {/if}
  {#each visible as p (p)}
    <div class="slot" class:hidden={p !== shown} {@attach dockSlot(p)}></div>
  {/each}
</div>

<style>
  .group {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .strip {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    padding: 0 6px 0 4px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
    flex-shrink: 0;
    min-width: 0;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    height: 24px;
    border-radius: 5px;
    color: var(--fg-muted);
  }
  .tab:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .tab.on {
    background: var(--bg-active);
    color: var(--fg);
  }
  .tab button[role="tab"] {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 100%;
    padding: 0 22px 0 8px;
  }
  .tab.fixed button[role="tab"] {
    padding-inline-end: 8px;
    border: 0;
    background: transparent;
    color: inherit;
    font-weight: 500;
    white-space: nowrap;
    cursor: default;
  }
  .close {
    position: absolute;
    inset-inline-end: 3px;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-muted);
    opacity: 0;
  }
  .tab:hover .close,
  .tab.on .close,
  .close:focus-visible {
    opacity: 1;
  }
  .close:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .slot {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .slot.hidden {
    display: none;
  }
</style>
