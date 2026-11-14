<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { t } from "./lib/i18n";
  import { isMac, keyLabel, mod } from "./lib/platform";
  import { agentNote, app } from "./lib/stores/app.svelte";
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
  import ChatConsent from "./lib/components/ChatConsent.svelte";
  import SidePanel from "./lib/components/SidePanel.svelte";
  import Viewer from "./lib/components/Viewer.svelte";
  import SearchPanel from "./lib/components/SearchPanel.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import TerminalDrawer from "./lib/components/TerminalDrawer.svelte";
  import Splitter from "./lib/components/Splitter.svelte";
  import ClaudeAccess from "./lib/components/ClaudeAccess.svelte";
  import AgentPane from "./lib/components/AgentPane.svelte";
  import PlanPanel from "./lib/components/PlanPanel.svelte";
  import DockGroup from "./lib/components/DockGroup.svelte";
  import DockOverlay from "./lib/components/DockOverlay.svelte";
  import { reportIssue, updates } from "./lib/updates.svelte";
  import { dock, dockHost } from "./lib/stores/dock.svelte";
  import PromptsPanel from "./lib/components/PromptsPanel.svelte";
  import { DEFAULT_BOTTOM, DEFAULT_WIDTH, MIN_WIDTH, fillGroup, shownPanel, type DockGroup as Group, type PanelId } from "./lib/dock";

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
    if ((app.overlay === "palette" || app.overlay === "new") && !app.agents.length) app.loadAgents();
  });

  // New session menu: chat or pane for each installed agent.
  const newItems = $derived.by<ListItem[]>(() => {
    const items: ListItem[] = [];
    for (const a of app.agents) {
      if (!a.path) continue;
      items.push({ id: `chat-${a.id}`, label: t("cmd.newSessionWith", { name: a.name }), detail: agentNote(a.id) || undefined, run: () => app.newSession(undefined, a.id, "chat") });
      if (a.tui) items.push({ id: `pane-${a.id}`, label: t("cmd.openPane", { name: a.name }), run: () => app.newSession(undefined, a.id, "pane") });
    }
    if (app.settings.agent === "custom")
      items.push({ id: "chat-custom", label: t("cmd.newSessionWith", { name: t("agent.custom") }), run: () => app.newSession(undefined, "custom", "chat") });
    return items;
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
      { id: "files", label: t("cmd.files"), hint: keyLabel("Mod+B"), run: () => dock.toggle("side") },
      { id: "term", label: t("cmd.terminal"), hint: keyLabel("Mod+`"), run: () => dock.toggle("terminal") },
      { id: "planPanel", label: t("cmd.planPanel"), run: () => dock.show("plan") },
      { id: "promptsPanel", label: t("cmd.promptsPanel"), run: () => dock.show("prompts") },
      { id: "sessionPanel", label: t("cmd.sessionPanel"), run: () => dock.show("session") },
      { id: "plan", label: t("cmd.planFirst"), run: () => app.setPlanFirst(!app.planFirst) },
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
        items.push({ id: `new-${a.id}`, label: t("cmd.newSessionWith", { name: a.name }), run: () => app.newSession(undefined, a.id, "chat") });
      if (a.path && a.tui)
        items.push({ id: `pane-${a.id}`, label: t("cmd.openPane", { name: a.name }), run: () => app.newSession(undefined, a.id, "pane") });
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

  // Docked panels (lib/dock). A group shows while one of its panels is visible.
  let winW = $state(window.innerWidth);
  const visible = (p: PanelId) => dock.visible(p);
  const groupsIn = (list: Group[]) => list.filter((g) => g.panels.some(visible));
  const leftGroups = $derived(groupsIn(app.layout.dock.left));
  const centerGroups = $derived(groupsIn(app.layout.dock.center));
  const rightGroups = $derived(groupsIn(app.layout.dock.right));
  const bottomGroups = $derived(groupsIn(app.layout.dock.bottom));
  // One center group fills the middle: the one with the Session panel, else the first. The
  // other groups have a width. Center groups before the fill group have their splitter on the
  // right, the ones after it on the left.
  const fill = $derived(fillGroup(centerGroups));
  const fillAt = $derived(fill ? centerGroups.indexOf(fill) : -1);
  const sideGroups = $derived([...leftGroups, ...centerGroups.filter((g) => g !== fill), ...rightGroups]);

  const minWidth = (g: Group) => Math.max(...g.panels.filter(visible).map((p) => MIN_WIDTH[p]));
  // The middle keeps the minimum width of the fill group (360px with the Session panel), and
  // 360px when the Session panel is at the bottom. A group takes what is left, within its own limits.
  const midMin = $derived(Math.max(fill ? minWidth(fill) : 160, bottomGroups.some((g) => g.panels.includes("session")) ? MIN_WIDTH.session : 0));
  const maxWidth = (g: Group) => () => {
    const others = sideGroups.reduce((sum, x) => (x === g ? sum : sum + Math.max(minWidth(x), x.size)), 0);
    return Math.max(minWidth(g), winW - 48 - midMin - others);
  };
  // When the groups do not fit, each one gives up width in proportion to its size, down to its minimum.
  const wanted = $derived(sideGroups.reduce((sum, x) => sum + Math.max(minWidth(x), x.size), 0));
  const room = $derived(winW - 48 - midMin);
  const widthOf = (g: Group) => {
    const want = Math.max(minWidth(g), g.size);
    return wanted <= room ? want : Math.max(minWidth(g), Math.floor((want * room) / wanted));
  };

  function resize(g: Group, v: number, done: boolean) {
    g.size = v;
    if (done) app.save();
  }
  function resetSize(g: Group) {
    g.size = DEFAULT_WIDTH[shownPanel(g, visible) ?? g.active];
    app.save();
  }

  // A panel that opens comes to the front of its group: a file in the viewer, the terminal, the
  // side bar, and the plan when a new plan arrives in the active session.
  let lastViewer: unknown = null;
  $effect(() => {
    const v = app.viewer;
    if (v && v !== lastViewer) untrack(() => dock.reveal("viewer"));
    lastViewer = v;
  });
  let lastTerminal = false;
  $effect(() => {
    const on = app.terminalVisible;
    if (on && !lastTerminal) untrack(() => dock.reveal("terminal"));
    lastTerminal = on;
  });
  let lastSide = true;
  $effect(() => {
    const on = app.filesVisible;
    if (on && !lastSide) untrack(() => dock.reveal("side"));
    lastSide = on;
  });
  const planSeen = new Map<string, string | undefined>();
  $effect(() => {
    const s = app.session;
    if (!s) return;
    const id = s.entries.findLast((e) => e.t === "plan" || e.t === "panePlan")?.id;
    const known = planSeen.has(s.key);
    const before = planSeen.get(s.key);
    planSeen.set(s.key, id);
    if (known && id && id !== before && !s.replaying) untrack(() => dock.open("plan"));
  });

  function inTerminal(el: Element | null) {
    return !!el?.closest(".xterm");
  }

  function onkeydown(e: KeyboardEvent) {
    const target = document.activeElement;
    // The terminal keeps its keys. Only Mod+`, Ctrl+Tab and Ctrl+Shift+key reach the app.
    if (inTerminal(target) && !(mod(e) && e.key === "`") && !(e.ctrlKey && e.shiftKey) && !(e.ctrlKey && e.key === "Tab")) return;
    const k = e.key.toLowerCase();
    if (mod(e) && !e.shiftKey && k === "k") app.overlay = app.overlay === "palette" ? null : "palette";
    else if (mod(e) && !e.shiftKey && k === "p") app.overlay = "openFile";
    else if (mod(e) && e.shiftKey && k === "f") app.overlay = "search";
    else if (mod(e) && k === "j") app.overlay = "queue";
    else if (mod(e) && k === "e") app.overlay = "sessions";
    else if (mod(e) && k === "b") dock.toggle("side");
    else if (mod(e) && e.key === "`") dock.toggle("terminal");
    else if (mod(e) && e.key === ".") app.stop();
    // Sessions: Ctrl+Tab and Ctrl+Shift+Tab cycle (Ctrl on every OS), Mod+1 to Mod+9 pick one.
    else if (e.ctrlKey && !e.altKey && e.key === "Tab") app.cycleSession(e.shiftKey ? -1 : 1);
    // Ctrl+Shift+Page Up and Page Down move the active tab (Ctrl on every OS, like Ctrl+Tab).
    else if (e.ctrlKey && e.shiftKey && !e.altKey && (e.key === "PageUp" || e.key === "PageDown")) app.moveActiveSession(e.key === "PageUp" ? -1 : 1);
    else if (mod(e) && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key) && !inTerminal(target)) app.showSessionAt(Number(e.key) - 1);
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
      {#if app.envProblem}
        <div class="mx-auto mt-6 w-[560px] max-w-[90vw]"><EnvBanner /></div>
      {/if}
      <div class="flex-1">
        <EmptyState recent={app.recent} onopen={() => app.pickFolder()} onopenpath={(p) => app.openPath(p)} />
      </div>
    </div>
  {:else}
    <div class="work" data-dock-work>
      {#each leftGroups as g (g.id)}
        <div class="col" style="width: {widthOf(g)}px"><DockGroup zone="left" group={g} /></div>
        <Splitter edge="right" value={g.size} min={minWidth(g)} max={maxWidth(g)} onchange={(v, d) => resize(g, v, d)} onreset={() => resetSize(g)} />
      {/each}
      <div class="mid" style="min-width: {midMin}px">
        <div class="top" data-dock-top>
          {#each centerGroups as g, i (g.id)}
            {#if g === fill}
              <div class="fill" data-dock-fill={g.id} style="min-width: {minWidth(g)}px"><DockGroup zone="center" group={g} /></div>
            {:else if i < fillAt}
              <div class="col" style="width: {widthOf(g)}px"><DockGroup zone="center" group={g} /></div>
              <Splitter edge="right" value={g.size} min={minWidth(g)} max={maxWidth(g)} onchange={(v, d) => resize(g, v, d)} onreset={() => resetSize(g)} />
            {:else}
              <Splitter edge="left" value={g.size} min={minWidth(g)} max={maxWidth(g)} onchange={(v, d) => resize(g, v, d)} onreset={() => resetSize(g)} />
              <div class="col" style="width: {widthOf(g)}px"><DockGroup zone="center" group={g} /></div>
            {/if}
          {:else}
            <div class="fill empty" data-dock-fill=""><p>{t("dock.emptyCenter")}</p></div>
          {/each}
        </div>
        {#if bottomGroups.length}
          <Splitter
            edge="top"
            value={app.layout.bottomHeight}
            min={120}
            max={() => window.innerHeight * 0.8}
            onchange={(v, d) => {
              app.layout.bottomHeight = v;
              if (d) app.save();
            }}
            onreset={() => {
              app.layout.bottomHeight = DEFAULT_BOTTOM;
              app.save();
            }}
          />
          <div class="bottom" style="height: {app.layout.bottomHeight}px">
            {#each bottomGroups as g (g.id)}
              <div class="cell" style="min-width: {g.panels.includes('session') ? MIN_WIDTH.session : 0}px"><DockGroup zone="bottom" group={g} /></div>
            {/each}
          </div>
        {/if}
      </div>
      {#each rightGroups as g (g.id)}
        <Splitter edge="left" value={g.size} min={minWidth(g)} max={maxWidth(g)} onchange={(v, d) => resize(g, v, d)} onreset={() => resetSize(g)} />
        <div class="col" style="width: {widthOf(g)}px"><DockGroup zone="right" group={g} /></div>
      {/each}
    </div>
    <!-- Each panel renders once here and moves into its slot in a dock group (dockHost). -->
    <div class="pool">
      <!-- The Session panel: the agent pane or the conversation of the active session. Moving it
           moves this element, so the pane's terminal and its screen stay as they are. -->
      <main class="host session" {@attach dockHost("session")}>
        <IdentityBar />
        {#if session && session.mode === "pane"}
          {#key session.key}<AgentPane {session} {project} />{/key}
        {:else if session}
          {#if session.status === "claudeAccess"}
            <div class="mx-auto mt-4 w-full max-w-[760px] px-6"><ClaudeAccess {session} /></div>
          {:else if app.envProblem}
            <div class="mx-auto mt-4 w-full max-w-[760px] px-6"><EnvBanner /></div>
          {/if}
          <Conversation bind:this={conversation} {session} />
          <MessageBox bind:this={messageBox} {session} onfocuscard={() => conversation?.focusLastCard()} />
        {:else}
          {#if app.envProblem}
            <div class="mx-auto mt-4 w-full max-w-[760px] px-6"><EnvBanner /></div>
          {/if}
          <div class="flex-1"></div>
        {/if}
      </main>
      {#if visible("side")}<div class="host" {@attach dockHost("side")}><SidePanel {project} /></div>{/if}
      {#if visible("viewer")}<div class="host" {@attach dockHost("viewer")}><Viewer {project} /></div>{/if}
      {#if visible("plan")}<div class="host" {@attach dockHost("plan")}><PlanPanel /></div>{/if}
      {#if visible("prompts")}<div class="host" {@attach dockHost("prompts")}><PromptsPanel /></div>{/if}
      {#if visible("terminal")}<div class="host" {@attach dockHost("terminal")}>{#key project.id}<TerminalDrawer {project} />{/key}</div>{/if}
    </div>
    {#if dock.dragging}<DockOverlay />{/if}
  {/if}
</div>

{#if app.overlay === "queue"}
  <ListOverlay title={t("queue.title")} items={queueItems} searchable={false} empty={t("queue.empty")} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "sessions"}
  <ListOverlay placeholder={t("session.list")} items={savedList} empty={t("session.listEmpty")} onclose={() => (app.overlay = null)} />
{:else if app.overlay === "new"}
  <ListOverlay placeholder={t("session.new")} items={newItems} empty={t("palette.empty")} onclose={() => (app.overlay = null)} />
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

{#if app.chatAsk}
  <ChatConsent name={app.agentName(app.chatAsk.agent)} onanswer={(choice) => app.chatAsk?.answer(choice)} />
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
  .work {
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .mid {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .top {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .fill {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .fill.empty {
    align-items: center;
    justify-content: center;
    color: var(--fg-faint);
  }
  .col {
    flex-shrink: 0;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bottom {
    flex-shrink: 0;
    display: flex;
    min-height: 0;
  }
  .cell {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .cell + .cell {
    border-inline-start: 1px solid var(--border);
  }
  .pool {
    display: none;
  }
  .host {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
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
