<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api, type HooksInfo } from "../api";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { applyTheme, type Theme } from "../theme.svelte";
  import { reportIssue, updates } from "../updates.svelte";

  const info = $derived({ version: app.env?.version, os: app.env?.os });
  const status = $derived.by(() => {
    const st = updates.state;
    if (st.kind === "checking") return t("update.checking");
    if (st.kind === "none") return t("update.none");
    if (st.kind === "downloading") return t("update.available", { version: st.version });
    if (st.kind === "ready") return t("update.readyHint", { version: st.version });
    if (st.kind === "installing") return t("update.installing", { version: st.version });
    if (st.kind === "error") return t("update.failed", { error: st.message });
    return "";
  });

  let { onclose }: { onclose: () => void } = $props();

  let shellName = $state("");
  let hooks = $state<HooksInfo | null>(null);
  let copied = $state<string | null>(null);
  $effect(() => {
    api.hooksInfo().then((h) => (hooks = h)).catch(() => {});
  });
  // FocusIDE never writes the Codex or Gemini CLI config; the user pastes the snippet.
  async function copy(which: "codex" | "gemini", text: string) {
    await writeText(text).catch(() => {});
    copied = which;
    setTimeout(() => (copied = null), 1500);
  }
  $effect(() => {
    app.loadAgents();
  });
  $effect(() => {
    invoke<string>("default_shell_name").then((s) => (shellName = s)).catch(() => {});
  });

  function setTheme(v: Theme) {
    app.settings.theme = v;
    applyTheme(v);
    app.save();
  }
  function setFont(n: number) {
    app.settings.fontSize = n;
    document.documentElement.style.setProperty("--read-size", `${n}px`);
    app.save();
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose} {onkeydown}>
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="settings-title">
    <h2 id="settings-title" class="mb-4 text-[15px] font-semibold">{t("settings.title")}</h2>
    <div class="rows">
      <label for="s-theme">{t("settings.theme")}</label>
      <select id="s-theme" value={app.settings.theme} onchange={(e) => setTheme(e.currentTarget.value as Theme)}>
        <option value="system">{t("settings.theme.system")}</option>
        <option value="light">{t("settings.theme.light")}</option>
        <option value="dark">{t("settings.theme.dark")}</option>
      </select>

      <label for="s-font">{t("settings.fontSize")}</label>
      <select id="s-font" value={String(app.settings.fontSize)} onchange={(e) => setFont(Number(e.currentTarget.value))}>
        {#each [13, 14, 15, 16, 17, 18] as n (n)}<option value={String(n)}>{n} px</option>{/each}
      </select>

      <label for="s-shell">{t("settings.shell")}</label>
      <input
        id="s-shell"
        value={app.settings.shell}
        placeholder={t("settings.shellDefault", { shell: shellName })}
        spellcheck="false"
        onchange={(e) => {
          app.settings.shell = e.currentTarget.value.trim();
          app.save();
        }}
      />

      <label for="s-agent">{t("settings.agent")}</label>
      <select
        id="s-agent"
        value={app.settings.agent}
        onchange={(e) => {
          app.settings.agent = e.currentTarget.value;
          app.save();
        }}
      >
        {#each app.agents as a (a.id)}
          <option value={a.id}>{a.path ? a.name : t("agent.notInstalled", { name: a.name })}</option>
        {/each}
        {#if !app.agents.length}<option value="claude">Claude</option>{/if}
        <option value="custom">{t("agent.custom")}</option>
      </select>

      {#if app.settings.agent === "custom"}
        <label for="s-agent-cmd">{t("agent.commandLabel")}</label>
        <input
          id="s-agent-cmd"
          value={app.settings.agentCommand}
          placeholder={t("agent.commandHint")}
          spellcheck="false"
          onchange={(e) => {
            app.settings.agentCommand = e.currentTarget.value.trim();
            app.save();
          }}
        />
      {/if}

      <label for="s-ignored">{t("settings.ignored")}</label>
      <input
        id="s-ignored"
        type="checkbox"
        checked={app.settings.showIgnored}
        onchange={(e) => {
          app.settings.showIgnored = e.currentTarget.checked;
          app.save();
        }}
      />

      <label for="s-notify">{t("settings.notifications")}</label>
      <input
        id="s-notify"
        type="checkbox"
        checked={app.settings.notifications}
        onchange={(e) => {
          app.settings.notifications = e.currentTarget.checked;
          app.save();
        }}
      />

      <label for="s-update">{t("update.auto")}</label>
      <input
        id="s-update"
        type="checkbox"
        checked={app.settings.autoUpdate}
        onchange={(e) => {
          app.settings.autoUpdate = e.currentTarget.checked;
          if (e.currentTarget.checked) updates.start(true);
          else updates.stop();
          app.save();
        }}
      />
    </div>

    <div class="hooks">
      <h3 class="mb-2 font-semibold">{t("hooks.title")}</h3>
      <label class="flex items-start gap-2">
        <input
          type="checkbox"
          class="mt-0.5"
          checked={app.settings.claudeHooks === true}
          onchange={(e) => {
            app.settings.claudeHooks = e.currentTarget.checked;
            app.save();
          }}
        />
        <span>
          {t("hooks.claude")}
          <span class="block text-fg-muted">{t("hooks.claudeHint")}</span>
        </span>
      </label>
      {#if hooks && !hooks.helper}
        <p class="mt-2 text-warn">{t("hooks.helperMissing")}</p>
      {/if}
      {#if hooks?.codex && hooks?.gemini}
        <details class="mt-3">
          <summary>{t("hooks.other")}</summary>
          <p class="mt-2 text-fg-muted">{t("hooks.otherHint")}</p>
          {#each [{ id: "codex" as const, label: t("hooks.codexFile"), text: hooks.codex }, { id: "gemini" as const, label: t("hooks.geminiFile"), text: hooks.gemini }] as snip (snip.id)}
            <div class="mt-3 flex items-center gap-2">
              <span class="flex-1">{snip.label}</span>
              <button class="btn h-6" onclick={() => copy(snip.id, snip.text)}>{copied === snip.id ? t("common.copied") : t("common.copy")}</button>
            </div>
            <pre class="snippet selectable">{snip.text}</pre>
          {/each}
        </details>
      {/if}
    </div>

    <div class="about">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-fg-muted">{t("help.version", { version: app.env?.version ?? "" })}</span>
        <span class="flex-1"></span>
        {#if updates.state.kind === "ready"}
          <button class="btn" onclick={() => updates.installAndRestart()}>{t("update.ready", { version: updates.state.version })}</button>
        {:else}
          <button class="btn" onclick={() => updates.check(true)} disabled={updates.state.kind === "checking" || updates.state.kind === "downloading"}>
            {t("update.check")}
          </button>
        {/if}
      </div>
      {#if status}<p class="mt-2 text-fg-muted selectable" role="status">{status}</p>{/if}
      <div class="mt-3 flex flex-wrap gap-2">
        <button class="btn" onclick={() => reportIssue("bug", info)}>{t("help.reportBug")}</button>
        <button class="btn" onclick={() => reportIssue("feature")}>{t("help.requestFeature")}</button>
      </div>
    </div>

    <div class="mt-5 flex justify-end">
      <button class="btn" onclick={onclose}>{t("common.close")}</button>
    </div>
  </div>
</div>

<style>
  .hooks {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .hooks summary {
    cursor: pointer;
  }
  .snippet {
    margin-top: 4px;
    max-height: 160px;
    overflow: auto;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-sunken);
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: pre;
  }
  .about {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 60;
    display: grid;
    place-items: center;
  }
  .dialog {
    width: 460px;
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 32px);
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 18px 20px 16px;
  }
  .rows {
    display: grid;
    grid-template-columns: 1fr 1.3fr;
    gap: 10px 16px;
    align-items: center;
  }
  select,
  input:not([type="checkbox"]) {
    height: 28px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
    padding: 0 8px;
    font: inherit;
    user-select: text;
  }
  input[type="checkbox"] {
    justify-self: start;
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
</style>
