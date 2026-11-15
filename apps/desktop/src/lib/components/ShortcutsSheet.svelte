<script lang="ts">
  // Keyboard shortcuts (Mod+/ or the palette): every shortcut from lib/keys.ts, by group.
  import { X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { GROUPS, SHORTCUTS } from "../keys";
  import { keyLabel } from "../platform";

  let { onclose }: { onclose: () => void } = $props();
  let panel: HTMLDivElement | undefined = $state();

  $effect(() => {
    queueMicrotask(() => panel?.focus());
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="panel" bind:this={panel} tabindex="-1" role="dialog" aria-label={t("keys.title")} onclick={(e) => e.stopPropagation()} {onkeydown}>
    <header class="head">
      <h2 class="font-semibold">{t("keys.title")}</h2>
      <span class="flex-1"></span>
      <button class="icon-btn" onclick={onclose} aria-label={t("common.close")} title={t("common.close")}><X size={14} /></button>
    </header>
    <div class="body selectable">
      {#each GROUPS as g (g.id)}
        <section>
          <h3>{t(g.label)}</h3>
          <dl>
            {#each SHORTCUTS.filter((s) => s.group === g.id) as s (s.id)}
              <dt>
                {t(s.label)}
                {#if s.where}<span class="where">{t(s.where)}</span>{/if}
              </dt>
              <dd>
                {#each s.keys as k, i (k)}{#if i}<span class="or">{t("keys.or")}</span>{/if}<span class="kbd">{keyLabel(k)}</span>{/each}
              </dd>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 50;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 8vh;
  }
  .panel {
    width: 720px;
    max-width: calc(100vw - 32px);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    overflow: hidden;
    outline: none;
  }
  .head {
    display: flex;
    align-items: center;
    padding: 10px 12px 8px 16px;
    border-bottom: 1px solid var(--border);
  }
  .body {
    overflow-y: auto;
    padding: 4px 16px 16px;
    columns: 2 300px;
    column-gap: 28px;
  }
  section {
    break-inside: avoid;
    padding-top: 12px;
  }
  h3 {
    margin-bottom: 4px;
    color: var(--fg-muted);
    font-size: 12px;
    font-weight: 600;
  }
  dl {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 4px 12px;
    align-items: baseline;
  }
  dt {
    min-width: 0;
  }
  .where {
    display: block;
    color: var(--fg-faint);
    font-size: 11.5px;
  }
  dd {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 4px;
  }
  .or {
    color: var(--fg-faint);
    font-size: 11.5px;
  }
</style>
