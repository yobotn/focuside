<script lang="ts">
  // Asks before a git action of the Git panel: discard changes (cannot be undone), or commit
  // all changes when nothing is staged. Esc cancels.
  import { t } from "../i18n";
  import type { GitAsk } from "../stores/git.svelte";

  let { ask }: { ask: GitAsk } = $props();

  let first: HTMLButtonElement | undefined = $state();

  $effect(() => {
    queueMicrotask(() => first?.focus());
  });

  const discard = $derived(ask.kind === "discard");
  const one = $derived(ask.paths.length === 1);

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      ask.answer("cancel");
    } else if (e.key === "Tab") {
      // Keep the focus in the dialog.
      const all = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("button")];
      const i = all.indexOf(document.activeElement as HTMLElement);
      const next = e.shiftKey ? (i <= 0 ? all.length - 1 : i - 1) : (i + 1) % all.length;
      e.preventDefault();
      all[next]?.focus();
    }
  }
</script>

<div class="backdrop">
  <div class="dialog" role="alertdialog" tabindex="-1" aria-modal="true" aria-labelledby="git-ask-title" aria-describedby="git-ask-body" {onkeydown}>
    {#if discard}
      <h2 id="git-ask-title" class="mb-2 text-[15px] font-semibold">
        {one ? t("git.discardOne", { name: ask.paths[0].split("/").pop() ?? ask.paths[0] }) : t("git.discardMany", { count: ask.paths.length })}
      </h2>
      <p id="git-ask-body" class="mb-2">
        {ask.deletes ? t("git.discardDeletes", { count: ask.deletes }) : t("git.discardBody")}
      </p>
      {#if !one}
        <ul class="files">
          {#each ask.paths as p (p)}
            <li class="mono truncate" title={p}>{p}</li>
          {/each}
        </ul>
      {/if}
      <div class="mt-3 flex justify-end gap-2">
        <button class="btn" bind:this={first} onclick={() => ask.answer("cancel")}>{t("common.cancel")}</button>
        <button class="btn btn-danger" onclick={() => ask.answer("yes")}>{t("git.discard")}</button>
      </div>
    {:else}
      <h2 id="git-ask-title" class="mb-2 text-[15px] font-semibold">{t("git.commitAllTitle")}</h2>
      <p id="git-ask-body" class="mb-2">{t("git.commitAllBody")}</p>
      <div class="mt-3 flex justify-end gap-2">
        <button class="btn" onclick={() => ask.answer("cancel")}>{t("common.cancel")}</button>
        <button class="btn" onclick={() => ask.answer("always")}>{t("git.commitAllAlways")}</button>
        <button class="btn btn-primary" bind:this={first} onclick={() => ask.answer("yes")}>{t("git.commitAll")}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 75;
    display: grid;
    place-items: center;
  }
  .dialog {
    width: 440px;
    max-width: calc(100vw - 32px);
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 18px 20px 16px;
    outline: none;
  }
  .files {
    max-height: 160px;
    overflow-y: auto;
    color: var(--fg-muted);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
</style>
