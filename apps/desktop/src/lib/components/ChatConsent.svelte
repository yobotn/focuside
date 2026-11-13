<script lang="ts">
  // The one-time note before chat mode with an agent whose vendor terms are unclear for
  // consumer logins (CHAT_CONFIRM in the app store). FocusIDE cannot read the login, so the user
  // decides. Escape cancels.
  import { t } from "../i18n";

  let { name, onanswer }: { name: string; onanswer: (choice: "chat" | "pane" | null) => void } = $props();
  let paneBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    paneBtn?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onanswer(null);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" {onkeydown}>
  <div class="dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="chat-title" aria-describedby="chat-body">
    <h2 id="chat-title" class="mb-2 text-[15px] font-semibold">{t("agent.chatConfirmTitle", { name })}</h2>
    <p id="chat-body" class="mb-2">{t("agent.chatConfirm", { name })}</p>
    <p class="mb-4 text-fg-muted">{t("agent.chatConfirmDetail")}</p>
    <div class="flex justify-end gap-2">
      <button class="btn" bind:this={paneBtn} onclick={() => onanswer("pane")}>{t("agent.chatConfirmPane")}</button>
      <button class="btn btn-primary" onclick={() => onanswer("chat")}>{t("agent.chatConfirmYes")}</button>
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
