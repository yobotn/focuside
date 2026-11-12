<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { X } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel } from "../platform";
  import { app, type Project } from "../stores/app.svelte";
  import { isDark } from "../theme.svelte";

  let { project }: { project: Project } = $props();

  let host: HTMLDivElement | undefined = $state();
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let id: string | null = null;
  let exited = $state(false);
  let unlisten: UnlistenFn[] = [];
  let ro: ResizeObserver | null = null;

  function colors() {
    const css = getComputedStyle(document.documentElement);
    const v = (n: string) => css.getPropertyValue(n).trim();
    return {
      background: v("--bg-sunken"),
      foreground: v("--fg"),
      cursor: v("--fg"),
      selectionBackground: isDark() ? "#3f3f46" : "#d4d4d8",
    };
  }

  function decode(b64: string): Uint8Array {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  async function start() {
    if (!term || !fit) return;
    exited = false;
    fit.fit();
    try {
      id = await invoke<string>("pty_open", {
        projectId: project.id,
        cols: term.cols,
        rows: term.rows,
        shell: app.settings.shell || null,
      });
      if (app.terminalCommand) {
        const cmd = app.terminalCommand;
        app.terminalCommand = null;
        setTimeout(() => id && invoke("pty_write", { id, data: cmd + "\r" }), 600);
      }
    } catch (e) {
      term.writeln(t("term.failed", { error: String(e) }));
    }
  }

  onMount(async () => {
    term = new Terminal({
      fontFamily: '"Cascadia Code", "Cascadia Mono", Consolas, monospace',
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true,
      allowProposedApi: false,
      theme: colors(),
      scrollback: 5000,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host!);
    term.onData((data) => {
      if (exited) {
        if (data === "\r") start();
        return;
      }
      if (id) invoke("pty_write", { id, data }).catch(() => {});
    });
    term.onResize(({ cols, rows }) => {
      if (id) invoke("pty_resize", { id, cols, rows }).catch(() => {});
    });
    unlisten.push(
      await listen<{ id: string; data: string }>("pty", (e) => {
        if (e.payload.id === id) term?.write(decode(e.payload.data));
      }),
      await listen<{ id: string }>("pty-exit", (e) => {
        if (e.payload.id === id) {
          id = null;
          exited = true;
          term?.writeln("\r\n" + t("term.exited"));
        }
      }),
    );
    ro = new ResizeObserver(() => fit?.fit());
    ro.observe(host!);
    await start();
    term.focus();
  });

  // A command queued while the drawer was already open (install, sign-in).
  $effect(() => {
    const cmd = app.terminalCommand;
    if (cmd && id) {
      app.terminalCommand = null;
      invoke("pty_write", { id, data: cmd + "\r" }).catch(() => {});
      term?.focus();
    }
  });

  onDestroy(() => {
    unlisten.forEach((f) => f());
    ro?.disconnect();
    if (id) invoke("pty_close", { id }).catch(() => {});
    term?.dispose();
  });

</script>

<section class="drawer" style="height: {app.layout.terminalHeight}px">
  <header class="head">
    <span class="font-medium">{t("term.title")}</span>
    <span class="kbd">{keyLabel("Mod+`")}</span>
    <span class="flex-1"></span>
    <button class="icon-btn" onclick={() => (app.terminalVisible = false)} aria-label={t("common.close")} title={t("common.close")}>
      <X size={13} />
    </button>
  </header>
  <div class="term" bind:this={host}></div>
</section>

<style>
  .drawer {
    position: relative;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-sunken);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 6px 0 12px;
    color: var(--fg-muted);
  }
  .term {
    flex: 1;
    min-height: 0;
    padding: 0 0 4px 10px;
  }
</style>
