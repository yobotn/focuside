<script lang="ts">
  import { t } from "../i18n";
  import type { Entry } from "../types";
  import { app } from "../stores/app.svelte";
  import { cleanTitle, type Session } from "../stores/session.svelte";

  let { session, entry }: { session: Session; entry: Extract<Entry, { t: "permission" }> } = $props();

  // The adapter's own options, in its order. Short labels for the common kinds.
  function label(o: { name: string; kind: string }) {
    if (o.kind === "allow_once" && /^yes$/i.test(o.name)) return t("perm.allow");
    if (o.kind === "reject_once" && /^no$/i.test(o.name)) return t("perm.deny");
    return o.name;
  }

  const title = $derived(cleanTitle(session.root, entry.title));
</script>

<div class="row" class:open={entry.state === "open"}>
  <p class="mb-2 selectable">
    <span class="text-fg-muted">{t("perm.title", { action: "" })}</span>
    <span class:mono={entry.kind === "execute"}>{title}</span>
  </p>
  {#if entry.state === "open"}
    <div class="flex flex-wrap gap-2">
      {#each entry.options as o (o.optionId)}
        <button
          class="btn"
          class:btn-primary={o.kind === "allow_once"}
          onclick={() => app.answer(session, entry, o.optionId)}
        >
          {label(o)}
        </button>
      {/each}
    </div>
  {:else}
    <p class="text-fg-muted">{entry.answer ?? (entry.state === "cancelled" ? t("conv.cancelled") : "")}</p>
  {/if}
</div>

<style>
  .row {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .row.open {
    border-color: color-mix(in srgb, var(--warn-line) 55%, var(--border));
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }
</style>
