<script lang="ts">
  import { api, errText } from "../api";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import type { UndoPreview } from "../types";

  let { target }: { target: { projectId: string; turnId: string; entryId: string; sessionKey: string } } = $props();

  let preview = $state<UndoPreview | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  /** Conflicts the user chose to revert anyway. */
  let revert = $state<Record<string, boolean>>({});
  let confirmBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    api
      .undoPreview(target.projectId, target.turnId)
      .then((p) => {
        preview = p;
        queueMicrotask(() => confirmBtn?.focus());
      })
      .catch((e) => (error = errText(e)));
  });

  const count = $derived(preview ? preview.files.length - preview.yourEdits.length : 0);

  async function confirm() {
    if (!preview || busy) return;
    busy = true;
    try {
      const anyway = Object.entries(revert)
        .filter(([, v]) => v)
        .map(([k]) => k);
      const out = await api.undo(target.projectId, target.turnId, anyway);
      app.applyUndo(target, out);
      app.undoTarget = null;
    } catch (e) {
      error = t("undo.failed", { error: errText(e) });
    } finally {
      busy = false;
    }
  }

  function close() {
    app.undoTarget = null;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  }

  function uncovered(n: { path: string; why: string }) {
    if (n.why === "nestedRepo") return t("changes.nestedRepo", { folder: n.path });
    if (n.why === "bigFile") return t("changes.bigFile", { path: n.path });
    if (n.why === "symlink") return t("changes.symlink", { path: n.path });
    return t("changes.locked", { path: n.path });
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={close} {onkeydown}>
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="undo-title">
    <h2 id="undo-title" class="mb-2 text-[15px] font-semibold">{t("undo.title")}</h2>
    {#if error}
      <p class="text-danger selectable">{error}</p>
    {:else if !preview}
      <p class="text-fg-muted">{t("changes.computing")}</p>
    {:else}
      <p class="mb-3">{t("undo.body", { count })}</p>

      {#if preview.conflicts.length}
        <p class="mb-1 font-semibold text-warn">{t("undo.conflicts")}</p>
        <ul class="mb-3">
          {#each preview.conflicts as c (c)}
            <li class="flex items-center gap-2 py-0.5">
              <span class="mono truncate">{c}</span>
              <span class="ms-auto flex shrink-0 gap-1">
                <button class="btn h-6" class:sel={!revert[c]} onclick={() => (revert[c] = false)}>{t("undo.keepMine")}</button>
                <button class="btn h-6" class:sel={!!revert[c]} onclick={() => (revert[c] = true)}>{t("undo.revertAnyway")}</button>
              </span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if preview.yourEdits.length}
        <p class="mb-3 text-fg-muted">{t("undo.yourEdits")} <span class="mono">{preview.yourEdits.join(", ")}</span></p>
      {/if}

      {#if preview.commands.length}
        <p class="mb-1 font-semibold">{t("undo.commands")}</p>
        <ul class="mb-3 max-h-32 overflow-y-auto">
          {#each preview.commands as c, i (i)}
            <li class="mono truncate text-fg-muted" title={c}>{c}</li>
          {/each}
        </ul>
      {/if}

      {#if preview.notCovered.length || preview.ignoredChanged.length}
        <p class="mb-1 font-semibold text-warn">{t("changes.notCovered")}</p>
        <ul class="mb-3 text-warn">
          {#each preview.notCovered as n (n.path)}
            <li class="truncate">{uncovered(n)}</li>
          {/each}
          {#each preview.ignoredChanged as p (p)}
            <li class="truncate">{t("changes.ignoredChanged", { path: p })}</li>
          {/each}
        </ul>
      {/if}
    {/if}

    <div class="mt-2 flex justify-end gap-2">
      <button class="btn" onclick={close}>{t("common.cancel")}</button>
      <button class="btn btn-primary" bind:this={confirmBtn} onclick={confirm} disabled={!preview || busy}>
        {t("undo.confirm")}
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
  .sel {
    background: var(--bg-active);
    border-color: var(--fg-faint);
  }
</style>
