<script lang="ts">
  // Shown when Claude Code is signed in with a Claude plan, or not signed in, and a chat session
  // was started anyway or asked for a login during a prompt. Chat mode runs through the Claude
  // Agent SDK, so it takes an API key or a cloud provider. A plan login is routed, not refused:
  // "Open Claude pane" starts the unmodified `claude` TUI in the center view, with its own
  // login. FocusIDE never types into it (session.route, agent::ClaudeRoute).
  // Credentials stay with Claude Code: FocusIDE never reads or stores them.
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";

  let { session }: { session: Session } = $props();

  const windows = $derived((app.env?.os ?? "windows") === "windows");
  const command = $derived(
    windows ? 'setx ANTHROPIC_API_KEY "sk-ant-..."' : '{ "env": { "ANTHROPIC_API_KEY": "sk-ant-..." } }',
  );
  let copied = $state(false);

  async function copy() {
    await writeText(command).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  const open = (url: string) => openUrl(url).catch(() => {});
</script>

<div class="panel" role="alert">
  <div class="flex flex-wrap items-center gap-3">
    {#if session.route === "pane"}
      <button class="btn btn-primary" onclick={() => app.openPaneInstead(session)}>{t("access.openPane")}</button>
    {/if}
    <span>{t("access.title")}</span>
  </div>

  <details class="mt-3">
    <summary>{t("access.keyTitle")}</summary>
    <ol class="mt-2 mb-2 list-decimal ps-5">
      <li>
        {t("access.keyStep1")}
        <button class="link" onclick={() => open("https://platform.claude.com/settings/keys")}>{t("access.console")}</button>
      </li>
      <li>
        {windows ? t("access.keyStep2Windows") : t("access.keyStep2Unix")}
        <div class="mt-1 flex items-center gap-2">
          <code class="cmd selectable truncate">{command}</code>
          <button class="btn h-6" onclick={copy}>{copied ? t("common.copied") : t("common.copy")}</button>
        </div>
      </li>
    </ol>
    <p class="mb-2 text-fg-muted">{t("access.keyNote")}</p>
    <p class="mb-3">
      {t("access.cloud")}
      <button class="link" onclick={() => open("https://code.claude.com/docs/en/third-party-integrations")}>{t("access.cloudDocs")}</button>
    </p>
    <div class="flex flex-wrap items-center gap-2">
      <button class="btn" onclick={() => app.resumeAgent(session)}>{t("access.again")}</button>
      <button class="btn" onclick={() => (app.overlay = "settings")}>{t("access.otherAgent")}</button>
    </div>
  </details>
</div>

<style>
  .panel {
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 12px 16px;
    background: var(--bg-subtle);
  }
  summary {
    cursor: pointer;
    color: var(--fg-muted);
  }
  .cmd {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-sunken);
    border: 1px solid var(--border);
    min-width: 0;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: inherit;
    text-decoration: underline;
    text-decoration-color: var(--fg-faint);
    text-underline-offset: 2px;
    cursor: pointer;
  }
</style>
