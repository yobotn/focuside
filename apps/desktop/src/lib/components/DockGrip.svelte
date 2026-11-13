<script lang="ts">
  // The move handle in a panel header. Drag it (or the header) to a dock zone, or click it for
  // a menu: Move to Left, Center, Right or Bottom, and Reset panel layout.
  import { Check, DotsSixVertical } from "phosphor-svelte";
  import { t } from "../i18n";
  import type { PanelId, Zone } from "../dock";
  import { app } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";
  import { panelName, zoneName } from "./dockNames";

  let { panel }: { panel: PanelId } = $props();

  let open = $state(false);
  let pos = $state({ x: 0, y: 0 });
  let button: HTMLButtonElement | undefined = $state();
  let menu: HTMLDivElement | undefined = $state();

  const places: Zone[] = ["left", "center", "right", "bottom"];
  const zone = $derived(dock.zoneOf(panel));

  function show(focusFirst: boolean) {
    const r = button!.getBoundingClientRect();
    const w = 200;
    pos = { x: Math.max(8, Math.min(r.right - w, window.innerWidth - w - 8)), y: r.bottom + 4 };
    open = true;
    queueMicrotask(() => {
      const items = menu?.querySelectorAll<HTMLElement>("[role^=menuitem]");
      if (!items?.length) return;
      // Keep the menu on screen when the panel sits at the bottom.
      const h = menu!.offsetHeight;
      if (pos.y + h > window.innerHeight - 8) pos = { ...pos, y: Math.max(8, r.top - h - 4) };
      (focusFirst ? items[0] : menu)?.focus();
    });
  }

  function hide(refocus = true) {
    open = false;
    if (refocus) button?.focus();
  }

  function pick(z: Zone) {
    hide();
    dock.move(panel, { kind: "zone", zone: z });
  }

  function onmenukey(e: KeyboardEvent) {
    const items = [...(menu?.querySelectorAll<HTMLElement>("[role^=menuitem]") ?? [])];
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") items[(i + 1) % items.length]?.focus();
    else if (e.key === "ArrowUp") items[(i - 1 + items.length) % items.length]?.focus();
    else if (e.key === "Home") items[0]?.focus();
    else if (e.key === "End") items[items.length - 1]?.focus();
    else if (e.key === "Escape") hide();
    else if (e.key === "Tab") hide(false);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (open && !menu?.contains(e.target as Node) && !button?.contains(e.target as Node)) hide(false);
  }}
/>

<button
  bind:this={button}
  class="icon-btn grip"
  aria-label={t("dock.grip", { panel: panelName(panel) })}
  title={t("dock.gripHint")}
  aria-haspopup="menu"
  aria-expanded={open}
  onpointerdown={(e) => dock.start(panel, e)}
  onclick={(e) => (open ? hide() : show(e.detail === 0))}
>
  <DotsSixVertical size={14} weight="bold" />
</button>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div bind:this={menu} class="menu" role="menu" tabindex="-1" aria-label={t("dock.moveTo")} style="left: {pos.x}px; top: {pos.y}px" onkeydown={onmenukey}>
    <p class="label" aria-hidden="true">{t("dock.moveTo")}</p>
    {#each places as z (z)}
      <button role="menuitemradio" aria-checked={zone === z} tabindex="-1" class="item" onclick={() => pick(z)}>
        <span class="check" aria-hidden="true">{#if zone === z}<Check size={12} weight="bold" />{/if}</span>
        {zoneName(z)}
      </button>
    {/each}
    <div class="sep" role="separator"></div>
    <button
      role="menuitem"
      tabindex="-1"
      class="item"
      onclick={() => {
        hide();
        app.resetLayout();
      }}
    >
      <span class="check" aria-hidden="true"></span>
      {t("cmd.resetLayout")}
    </button>
  </div>
{/if}

<style>
  .grip {
    width: 22px;
    cursor: grab;
    color: var(--fg-faint);
  }
  .grip:hover,
  .grip[aria-expanded="true"] {
    color: var(--fg);
  }
  .menu {
    position: fixed;
    z-index: 70;
    width: 200px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    box-shadow: var(--shadow);
    outline: none;
  }
  .label {
    padding: 4px 8px 2px;
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-muted);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--fg);
    text-align: start;
  }
  .item:hover,
  .item:focus-visible {
    background: var(--bg-hover);
    outline: none;
  }
  .check {
    display: inline-grid;
    place-items: center;
    width: 14px;
    color: var(--accent);
  }
  .sep {
    height: 1px;
    margin: 4px 0;
    background: var(--border);
  }
</style>
