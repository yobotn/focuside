<script lang="ts">
  // The status bar at the bottom of the window. It shows the items of stores/statusbar.svelte.ts;
  // other parts of the app add their own there. Settings and the palette turn it on or off.
  import { t } from "../i18n";
  import { statusBar, type StatusItem } from "../stores/statusbar.svelte";

  const start = $derived(statusBar.side("start"));
  const end = $derived(statusBar.side("end"));
</script>

{#snippet item(it: StatusItem)}
  {@const text = it.text()}
  {#if text}
    {@const Icon = it.icon?.()}
    {@const tone = it.tone?.()}
    {#if it.run}
      <button class="item {tone ?? ''}" title={it.title?.()} onclick={() => it.run?.()}>
        {#if Icon}<Icon size={12} />{/if}{text}
      </button>
    {:else}
      <span class="item {tone ?? ''}" title={it.title?.()}>{#if Icon}<Icon size={12} />{/if}{text}</span>
    {/if}
  {/if}
{/snippet}

<footer class="status" aria-label={t("status.label")}>
  {#each start as it (it.id)}{@render item(it)}{/each}
  <span class="flex-1"></span>
  {#each end as it (it.id)}{@render item(it)}{/each}
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 24px;
    flex-shrink: 0;
    padding: 0 8px;
    border-top: 1px solid var(--border);
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-size: 12px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 6px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-variant-numeric: tabular-nums;
    min-width: 0;
  }
  button.item:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .accent {
    color: var(--accent);
  }
  .warn {
    color: var(--warn);
  }
  .danger {
    color: var(--danger);
  }
</style>
