<script lang="ts">
  // Shown when Claude Code is signed in with a Claude plan, or not signed in. Chat mode runs
  // through the Claude Agent SDK, so it takes an API key or a cloud provider. A plan login is
  // routed, not refused: the user runs the unmodified `claude` in the terminal drawer, with its
  // own login. FocusIDE opens the drawer but never types into it (session.route, agent::ClaudeRoute).
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
  <p class="mb-2 font-semibold">{t("access.title")}</p>
  <p class="mb-3">{session.access === "signedOut" ? t("access.signedOut") : t("access.subscription")}</p>

  {#if session.route === "terminal"}
    <p class="mb-1 font-medium">{t("access.planTitle")}</p>
    <p class="mb-2">{t("access.planBody")}</p>
    <div class="mb-2 flex items-center gap-2">
      <button class="btn" onclick={() => app.openTerminal()}>{t("access.openTerminal")}</button>
    </div>
    <p class="mb-3 text-fg-muted">{t("access.planNote")}</p>
  {/if}

  <p class="mb-1 font-medium">{t("access.keyTitle")}</p>
  <ol class="mb-2 list-decimal ps-5">
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
  <p class="mb-3 text-fg-muted">{t("access.keyNote")}</p>

  <p class="mb-1 font-medium">{t("access.cloudTitle")}</p>
  <p class="mb-3">
    {t("access.cloudBody")}
    <button class="link" onclick={() => open("https://code.claude.com/docs/en/third-party-integrations")}>{t("access.cloudDocs")}</button>
  </p>

  <div class="flex flex-wrap items-center gap-2">
    <button class="btn" onclick={() => app.resumeAgent(session)}>{t("access.again")}</button>
    <button class="btn" onclick={() => (app.overlay = "settings")}>{t("access.otherAgent")}</button>
    <span class="flex-1"></span>
    <button class="link" onclick={() => open("https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use")}>
      {t("access.why")}
    </button>
  </div>
</div>

<style>
  .panel {
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 14px 16px;
    background: var(--bg-subtle);
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
