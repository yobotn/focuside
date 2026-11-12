<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "../i18n";
  import { app } from "../stores/app.svelte";
  import { applyTheme, type Theme } from "../theme.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let shellName = $state("");
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
        {#if !app.agents.length}<option value="claude">Claude Code</option>{/if}
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
    </div>
    <div class="mt-5 flex justify-end">
      <button class="btn" onclick={onclose}>{t("common.close")}</button>
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
    width: 460px;
    max-width: calc(100vw - 32px);
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
