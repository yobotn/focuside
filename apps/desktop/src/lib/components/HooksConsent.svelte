<script lang="ts">
  // The one-time question before the first Claude pane: may FocusIDE start Claude panes with
  // its observe-only plugin (--plugin-dir)? The answer is stored in Settings, where it can be
  // changed. Escape answers no.
  import { t } from "../i18n";

  let { onanswer }: { onanswer: (yes: boolean) => void } = $props();
  let noBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    noBtn?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onanswer(false);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" {onkeydown}>
  <div class="dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="hooks-title" aria-describedby="hooks-body">
    <h2 id="hooks-title" class="mb-2 text-[15px] font-semibold">{t("hooks.consentTitle")}</h2>
    <p id="hooks-body" class="mb-2">{t("hooks.consent")}</p>
    <p class="mb-4 text-fg-muted">{t("hooks.consentDetail")}</p>
    <div class="flex justify-end gap-2">
      <button class="btn" bind:this={noBtn} onclick={() => onanswer(false)}>{t("hooks.deny")}</button>
      <button class="btn btn-primary" onclick={() => onanswer(true)}>{t("hooks.allow")}</button>
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
    width: 480px;
    max-width: calc(100vw - 32px);
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 18px 20px 16px;
  }
</style>
