<script lang="ts">
  // "Undo back to this prompt": lists the later prompts and the files they changed, then
  // undoes those turns one by one, newest first (app.undoBack).
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { firstLine } from "../prompts";

  let { target }: { target: NonNullable<typeof app.undoBackTarget> } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let confirmBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    queueMicrotask(() => confirmBtn?.focus());
  });

  async function confirm() {
    if (busy) return;
    busy = true;
    error = await app.undoBack();
    busy = false;
  }

  function close() {
    if (!busy) app.undoBackTarget = null;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={close} {onkeydown}>
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="undo-back-title">
    <h2 id="undo-back-title" class="mb-2 text-[15px] font-semibold">{t("undoBack.title")}</h2>
    <p class="mb-2">{t("undoBack.body", { count: target.turns.length })}</p>
    <ul class="mb-3 max-h-32 overflow-y-auto">
      {#each target.turns as p (p.id)}
        <li class="truncate text-fg-muted" title={p.prompt}>{firstLine(p.prompt) || t("prompts.empty")}</li>
      {/each}
    </ul>
    <p class="mb-1 font-semibold">{t("undoBack.files", { count: target.files.length })}</p>
    <ul class="mb-3 max-h-40 overflow-y-auto">
      {#each target.files as f (f)}
        <li class="mono truncate" title={f}>{f}</li>
      {/each}
    </ul>
    <p class="mb-3 text-fg-muted">{t("undoBack.kept")}</p>
    {#if error}<p class="mb-2 text-danger selectable">{error}</p>{/if}
    <div class="mt-2 flex justify-end gap-2">
      <button class="btn" onclick={close} disabled={busy}>{t("common.cancel")}</button>
      <button class="btn btn-primary" bind:this={confirmBtn} onclick={confirm} disabled={busy}>
        {busy ? t("undoBack.working") : t("undoBack.confirm")}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 60;
    display: grid;
    place-items: center;
  }
  .dialog {
    width: 520px;
    max-width: calc(100vw - 32px);
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 18px 20px 16px;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
</style>
