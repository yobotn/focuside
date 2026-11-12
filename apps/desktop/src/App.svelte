<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { t } from "./lib/i18n";
  import { isMac, keyLabel, mod } from "./lib/platform";
  import { app } from "./lib/stores/app.svelte";
  import { applyTheme } from "./lib/theme.svelte";
  import { allFiles } from "./lib/files.svelte";
  import Conversation from "./lib/components/Conversation.svelte";
  import EmptyState from "./lib/components/EmptyState.svelte";
  import EnvBanner from "./lib/components/EnvBanner.svelte";
  import IdentityBar from "./lib/components/IdentityBar.svelte";
  import ListOverlay, { type ListItem } from "./lib/components/ListOverlay.svelte";
  import MessageBox from "./lib/components/MessageBox.svelte";
  import Rail from "./lib/components/Rail.svelte";
  import UndoDialog from "./lib/components/UndoDialog.svelte";
  import FileTree from "./lib/components/FileTree.svelte";
  import Viewer from "./lib/components/Viewer.svelte";
  import SearchPanel from "./lib/components/SearchPanel.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import TerminalDrawer from "./lib/components/TerminalDrawer.svelte";
  import Splitter from "./lib/components/Splitter.svelte";
  import ClaudeAccess from "./lib/components/ClaudeAccess.svelte";
  import { reportIssue, updates } from "./lib/updates.svelte";
  import { defaultLayout, type Side } from "./lib/stores/app.svelte";

  let conversation: Conversation | undefined = $state();
  let messageBox: MessageBox | undefined = $state();
  let files = $state<string[]>([]);
  let savedList = $state<ListItem[]>([]);

  onMount(() => {
    app.init();
    const un = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") {
        // Folders open as projects. Files are left to the message box (images).
        for (const p of e.payload.paths) app.openPath(p, { quiet: true });
      }
    });
    return () => {
      un.then((f) => f());
    };
  });

  const project = $derived(app.active);
  const session = $derived(app.session);
  const projectColor = $derived(project ? `hsl(${project.hue} var(--project-s) var(--project-l))` : "var(--fg-faint)");

  // Overlays that need data.
  $effect(() => {
    if (app.overlay === "palette" && !app.agents.length) app.loadAgents();
  });
  $effect(() => {
    if (app.overlay === "openFile" && project) allFiles(project.id, true).then((f) => (files = f));
  });
  $effect(() => {
    if (app.overlay === "sessions" && project) {
      const saved = app.savedSessions[project.root] ?? [];
      const live = project.sessions;
      const items: ListItem[] = live.map((s) => ({
        id: s.key,
        label: s.name || t("session.untitled"),
        detail: s.running ? t("conv.working") : s.waiting ? t(`queue.${s.waiting}` as any) : "",
        tone: s.waiting ? "warn" : undefined,
        run: () => app.showSession(s.key),
      }));
      for (const s of saved) {
        if (live.some((l) => l.acpId === s.acpId)) continue;
        items.push({
          id: s.acpId,
          label: s.name || t("session.untitled"),
          detail: new Date(s.updated).toLocaleString(),
          tone: "muted",
          run: () => app.newSession(s),
        });
      }
      savedList = items;
    }
  });

  const queueItems = $derived<ListItem[]>(
    app.queue.map(({ project: p, session: s }) => ({
      id: s.key,
      label: `${p.name}  .  ${s.name || t("session.untitled")}`,
      detail: t(`queue.${s.waiting}` as any),
      hue: p.hue,
      tone: s.waiting === "plan" || s.waiting === "permission" ? "warn" : undefined,
      run: () => app.showSession(s.key),
    })),
  );

  const modelItems = $derived<ListItem[]>(
    (session?.model?.options ?? []).map((o) => ({
      id: o.value,
      label: o.name,
      detail: o.description ?? "",
      hint: o.value === session?.model?.currentValue ? t("model.current") : "",
      run: async () => {
        if (!session) return;
        try {
          await (await import("./lib/api")).api.sessionSetConfig(session.key, session.model!.id, o.value);
        } catch (e) {
          app.flash(String(e), "error");
        }
      },
    })),
  );

  const paletteItems = $derived.by<ListItem[]>(() => {
    const s = session;
    const items: ListItem[] = [
      { id: "open", label: t("cmd.openFolder"), run: () => app.pickFolder() },
      { id: "new", label: t("cmd.newSession"), run: () => app.newSession() },
      { id: "sessions", label: t("cmd.sessions"), hint: keyLabel("Mod+E"), run: () => (app.overlay = "sessions") },
      { id: "queue", label: t("cmd.waiting"), hint: keyLabel("Mod+J"), run: () => (app.overlay = "queue") },
      { id: "file", label: t("cmd.openFile"), hint: keyLabel("Mod+P"), run: () => (app.overlay = "openFile") },
      { id: "search", label: t("cmd.search"), hint: keyLabel("Mod+Shift+F"), run: () => (app.overlay = "search") },
      { id: "files", label: t("cmd.files"), hint: keyLabel("Mod+B"), run: () => toggleFiles() },
      { id: "term", label: t("cmd.terminal"), hint: keyLabel("Mod+`"), run: () => (app.terminalVisible = !app.terminalVisible) },
      { id: "plan", label: t("cmd.planFirst"), run: () => app.setPlanFirst(!app.planFirst) },
      { id: "moveFiles", label: t("cmd.moveFiles"), run: () => app.movePanel("files") },
      { id: "moveViewer", label: t("cmd.moveViewer"), run: () => app.movePanel("viewer") },
      { id: "resetLayout", label: t("cmd.resetLayout"), run: () => app.resetLayout() },
      {
        id: "update",
        label: t("update.check"),
        run: () => {
          app.overlay = "settings";
          updates.check(true);
        },
      },
      { id: "bug", label: t("help.reportBug"), run: () => reportIssue("bug", { version: app.env?.version, os: app.env?.os }) },
      { id: "feature", label: t("help.requestFeature"), run: () => reportIssue("feature") },
      { id: "settings", label: t("cmd.settings"), run: () => (app.overlay = "settings") },
      {
        id: "theme",
        label: t("cmd.theme"),
        run: () => {
          const next = app.settings.theme === "system" ? "light" : app.settings.theme === "light" ? "dark" : "system";
          app.settings.theme = next;
          applyTheme(next);
          app.save();
        },
      },
    ];
    for (const a of app.agents) {
      if (a.path && a.id !== app.settings.agent)
        items.push({ id: `new-${a.id}`, label: t("cmd.newSessionWith", { name: a.name }), run: () => app.newSession(undefined, a.id) });
    }
    if (s?.running) items.unshift({ id: "stop", label: t("cmd.stop"), hint: keyLabel("Mod+."), run: () => app.stop(s) });
    const lastDone = s?.entries.findLast((e) => (e.t === "changes" || (e.t === "plan" && ["done", "paused", "failed"].includes(e.state))) && !e.undo);
    if (s && lastDone && (lastDone.t === "changes" || lastDone.t === "plan"))
      items.push({ id: "undo", label: t("cmd.undoTurn"), run: () => app.askUndo(s, lastDone) });
    if (app.lastUndo) items.push({ id: "redo", label: t("cmd.redoUndo"), run: () => app.redo() });
    if (project) items.push({ id: "close", label: t("cmd.closeProject"), run: () => app.closeProject(project.id) });
    return items;
  });

  const fileItems = $derived<ListItem[]>(
    files.map((f) => ({
      id: f,
      label: f,
      file: f,
      run: () => project && (app.viewer = { kind: "file", projectId: project.id, path: f }),
    })),
  );

  // Panels by side. Two panels on one side: the file tree sits on the outside.
  type Panel = "files" | "viewer";
  let winW = $state(window.innerWidth);
  const showViewer = $derived(!!(project && app.viewer && app.viewer.projectId === project.id));
  const panelsOn = (side: Side): Panel[] => {
    const out: Panel[] = [];
    if (app.filesVisible && app.layout.filesSide === side) out.push("files");
    if (showViewer && app.layout.viewerSide === side) out.push("viewer");
    return side === "left" ? out : out.reverse();
  };
  const leftPanels = $derived(panelsOn("left"));
  const rightPanels = $derived(panelsOn("right"));

  // The conversation keeps at least 360px; a panel takes what is left, within its own limits.
  const maxWidth = (panel: Panel) => () => {
    const other = panel === "files" ? (showViewer ? app.layout.viewerWidth : 0) : app.filesVisible ? app.layout.filesWidth : 0;
    return Math.max(panel === "files" ? 160 : 320, winW - 48 - 360 - other);
  };
  const widthKey = (panel: Panel) => (panel === "files" ? "filesWidth" : "viewerWidth") as "filesWidth" | "viewerWidth";

  function resize(panel: Panel, v: number, done: boolean) {
    app.layout[widthKey(panel)] = v;
    if (done) app.save();
  }

  function toggleFiles() {
    app.filesVisible = !app.filesVisible;
    app.save();
  }

  function inTerminal(el: Element | null) {
    return !!el?.closest(".xterm");
  }

  function onkeydown(e: KeyboardEvent) {
    const target = document.activeElement;
    // The terminal keeps its keys. Only Mod+` and Ctrl+Shift+key reach the app.
    if (inTerminal(target) && !(mod(e) && e.key === "`") && !(e.ctrlKey && e.shiftKey)) return;
    const k = e.key.toLowerCase();
    if (mod(e) && !e.shiftKey && k === "k") app.overlay = app.overlay === "palette" ? null : "palette";
    else if (mod(e) && !e.shiftKey && k === "p") app.overlay = "openFile";
    else if (mod(e) && e.shiftKey && k === "f") app.overlay = "search";
    else if (mod(e) && k === "j") app.overlay = "queue";
    else if (mod(e) && k === "e") app.overlay = "sessions";
    else if (mod(e) && k === "b") toggleFiles();
    else if (mod(e) && e.key === "`") app.terminalVisible = !app.terminalVisible;
    else if (mod(e) && e.key === ".") app.stop();
    else if (mod(e) && e.key === "Enter" && !(target instanceof HTMLTextAreaElement)) {
      const plan = session?.waitingPlan;
      if (session && plan) app.runPlan(session, plan);
    } else if (e.key === "Escape") {
      // Esc only closes overlays and the viewer. It never discards a plan.
      if (app.undoTarget) app.undoTarget = null;
      else if (app.overlay) app.overlay = null;
      else if (app.viewer) app.viewer = null;
      else if (session?.running) app.stop();
      else return;
    } else return;
    e.preventDefault();
  }
</script>

<svelte:window {onkeydown} bind:innerWidth={winW} />

<div class="app" class:mac={isMac} style="--project: {projectColor}">
  {#if app.projects.length}
    <Rail />
  {/if}

  {#if !app.ready}
    <div class="flex-1"></div>
  {:else if !project}
    <div class="flex flex-1 flex-col">
      {#if app.env && (!app.env.claude || !app.env.git || !app.env.adapter)}
        <div class="mx-auto mt-6 w-[560px] max-w-[90vw]"><EnvBanner /></div>
      {/if}
      <div class="flex-1">
        <EmptyState recent={app.recent} onopen={() => app.pickFolder()} onopenpath={(p) => app.openPath(p)} />
      </div>
    </div>
  {:else}
    {#snippet panel(p: Panel, edge: "left" | "right")}
      {#if edge === "left"}
        <Splitter edge="left" value={app.layout[widthKey(p)]} min={p === "files" ? 160 : 320} max={maxWidth(p)} onchange={(v, d) => resize(p, v, d)} onreset={() => resize(p, defaultLayout[widthKey(p)], true)} />
      {/if}
      <div class="panel" style="width: {Math.min(app.layout[widthKey(p)], maxWidth(p)())}px">
        {#if p === "files"}<FileTree {project} />{:else}<Viewer {project} />{/if}
      </div>
      {#if edge === "right"}
        <Splitter edge="right" value={app.layout[widthKey(p)]} min={p === "files" ? 160 : 320} max={maxWidth(p)} onchange={(v, d) => resize(p, v, d)} onreset={() => resize(p, defaultLayout[widthKey(p)], true)} />
      {/if}
    {/snippet}
    {#each leftPanels as p (p)}{@render panel(p, "right")}{/each}
    <main class="center">
      <IdentityBar />
      {#if session}
        {#if session.status === "claudeAccess"}
          <div class="mx-auto mt-4 w-full max-w-[760px] px-6"><ClaudeAccess {session} /></div>
        {:else if app.env && (!app.env.claude || !app.env.git || !app.env.adapter)}
          <div class="mx-auto mt-4 w-full max-w-[760px] px-6"><EnvBanner /></div>
        {/if}
        <Conversation bind:this={conversation} {session} />
        <MessageBox bind:this={messageBox} {session} onfocuscard={() => conversation?.focusLastCard()} />
      {:else}
        <div class="flex-1"></div>
      {/if}
      {#if app.terminalVisible}
        <Splitter
          edge="top"
          value={app.layout.terminalHeight}
          min={120}
          max={() => window.innerHeight * 0.8}
          onchange={(v, d) => {
            app.layout.terminalHeight = v;
            if (d) app.save();
          }}
          onreset={() => {
            app.layout.terminalHeight = defaultLayout.terminalHeight;
            app.save();
          }}
        />
        {#key project.id}<TerminalDrawer {project} />{/key}
      {/if}
    </main>
    {#each rightPanels as p (p)}{@render panel(p, "left")}{/each}
  {/if}
</div>

{#if app.overlay === "queue"}
  <ListOverlay title={t("queue.title")} items={queueItems} searchable={false} empty={t("queue.empty")} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "sessions"}
  <ListOverlay placeholder={t("session.list")} items={savedList} empty={t("session.listEmpty")} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "palette"}
  <ListOverlay placeholder={t("palette.placeholder")} items={paletteItems} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "openFile"}
  <ListOverlay placeholder={t("palette.openFile")} items={fileItems} mono onclose={() => (app.overlay = null)} />
{:else if app.overlay === "model"}
  <ListOverlay title={t("model.title")} items={modelItems} searchable={false} empty={t("model.none")} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "search" && project}
  <SearchPanel {project} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "settings"}
  <Settings onclose={() => (app.overlay = null)} />
{/if}

{#if app.undoTarget}
  <UndoDialog target={app.undoTarget} />
{/if}

{#if app.toast}
  <div class="toast" class:error={app.toast.tone === "error"} role="status">{app.toast.text}</div>
{/if}

<style>
  .app {
    display: flex;
    height: 100%;
    background: var(--bg);
  }
  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .panel {
    flex-shrink: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    max-width: 70vw;
    padding: 8px 14px;
    border-radius: 8px;
    background: var(--fg);
    color: var(--bg);
    box-shadow: var(--shadow);
    z-index: 80;
  }
  .toast.error {
    background: var(--danger);
    color: #fff;
  }
</style>
