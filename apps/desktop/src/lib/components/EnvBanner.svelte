<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api } from "../api";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";

  let { signin = false }: { signin?: boolean } = $props();

  const os = $derived(app.env?.os ?? "windows");
  const rows = $derived.by(() => {
    const out: { text: string; command: string }[] = [];
    const env = app.env;
    if (!env) return out;
    if (!env.claude)
      out.push({
        text: t("check.claudeMissing"),
        command: os === "windows" ? "irm https://claude.ai/install.ps1 | iex" : "curl -fsSL https://claude.ai/install.sh | bash",
      });
    if (!env.git)
      out.push({
        text: t("check.gitMissing"),
        command: os === "windows" ? "winget install --id Git.Git -e" : os === "macos" ? "xcode-select --install" : "sudo apt install git",
      });
    if (signin) out.push({ text: t("check.signInBody"), command: "claude" });
    return out;
  });

  const dl = $derived(app.env?.adapterDownload ?? null);

  let copied = $state<string | null>(null);

  async function copy(cmd: string) {
    await writeText(cmd).catch(() => {});
    copied = cmd;
    setTimeout(() => (copied = null), 1500);
  }

  function runInTerminal(cmd: string) {
    app.runInTerminal(cmd);
  }

  async function again() {
    app.env = await api.envCheck();
  }

  const showAdapter = $derived(!signin && app.needsAdapter);
</script>

{#if rows.length || showAdapter}
  <div class="banner">
    <p class="mb-2 font-semibold">{signin ? t("check.signIn") : t("check.title")}</p>
    {#each rows as r (r.text)}
      <div class="row">
        <p class="mb-1">{r.text}</p>
        {#if r.command}
          <div class="flex items-center gap-2">
            <code class="cmd selectable truncate">{r.command}</code>
            <button class="btn h-6" onclick={() => copy(r.command)}>{copied === r.command ? t("common.copied") : t("common.copy")}</button>
            <button class="btn h-6" onclick={() => runInTerminal(r.command)}>{t("check.runInTerminal")}</button>
          </div>
        {/if}
      </div>
    {/each}
    {#if showAdapter}
      <div class="row">
        {#if dl}
          <p class="mb-1">{t("check.adapterDownload", { repo: dl.repo, version: dl.version, size: dl.sizeMb })}</p>
          <p class="mb-1 muted">{t("check.adapterNote")}</p>
          <div class="flex items-center gap-2">
            <button class="btn h-6" disabled={app.adapterProgress !== null} onclick={() => app.downloadAdapter()}>
              {app.adapterProgress === null ? t("check.download") : t("check.downloading", { percent: app.adapterProgress })}
            </button>
          </div>
          {#if app.adapterError}
            <p class="mt-1 error selectable">{t("check.downloadFailed", { message: app.adapterError })}</p>
          {/if}
        {:else}
          <p class="mb-1">{t("check.adapterUnsupported")}</p>
        {/if}
      </div>
    {/if}
    {#if !signin}
      <button class="btn mt-2" onclick={again}>{t("check.again")}</button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    border: 1px solid color-mix(in srgb, var(--warn-line) 55%, var(--border));
    background: var(--warn-soft);
    border-radius: 8px;
    padding: 12px 14px;
  }
  .row + .row {
    margin-top: 10px;
  }
  .muted {
    color: var(--fg-muted);
  }
  .error {
    color: var(--danger);
  }
  .cmd {
    font-family: var(--font-mono);
    font-size: 12.5px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    min-width: 0;
  }
</style>
