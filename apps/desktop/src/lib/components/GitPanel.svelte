<script lang="ts">
  // The Git panel (dock id "git"), like VS Code's source control: the branch with ahead and
  // behind, Fetch, Pull and Push, the commit message box, the changes in groups (merge
  // conflicts, staged, changes, untracked) with stage, unstage and discard, and the last 50
  // commits. A click on a file opens its diff in the viewer. State lives in stores/git.svelte.ts.
  import {
    ArrowCounterClockwise,
    ArrowDown,
    ArrowsClockwise,
    ArrowUp,
    CaretDown,
    CaretRight,
    CloudArrowDown,
    File as FileIcon2,
    GitBranch,
    Minus,
    Plus,
    X,
  } from "phosphor-svelte";
  import type { GitCommit, GitCommitFile, GitEntry } from "../api";
  import { t } from "../i18n";
  import { keyLabel, mod } from "../platform";
  import { app, type Project } from "../stores/app.svelte";
  import { dock } from "../stores/dock.svelte";
  import { git as g, letterOf, type GitGroup } from "../stores/git.svelte";
  import DockGrip from "./DockGrip.svelte";
  import FileIcon from "./FileIcon.svelte";

  let { project }: { project: Project } = $props();

  let box: HTMLTextAreaElement | undefined = $state();
  let folded = $state<Record<string, boolean>>({ history: false });
  let openCommit = $state<string | null>(null);

  const st = $derived(g.projectId === project.id ? g.status : null);
  const message = $derived(g.messages[project.id] ?? "");
  const nothingStaged = $derived(!g.staged.length);

  // Palette "Commit" with an empty message: focus the box.
  let lastFocus = 0;
  $effect(() => {
    const n = g.focusMessage;
    if (n !== lastFocus) {
      lastFocus = n;
      queueMicrotask(() => box?.focus());
    }
  });

  const groups = $derived<{ id: GitGroup; label: string; list: GitEntry[] }[]>(
    [
      { id: "conflicts" as const, label: t("git.conflicts"), list: g.conflicts },
      { id: "staged" as const, label: t("git.staged"), list: g.staged },
      { id: "unstaged" as const, label: t("git.changes"), list: g.unstaged },
      { id: "untracked" as const, label: t("git.untracked"), list: g.untracked },
    ].filter((x) => x.list.length),
  );

  function onboxkey(e: KeyboardEvent) {
    if (mod(e) && e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      g.commit();
    }
  }

  function name(p: string) {
    return p.split("/").pop() ?? p;
  }
  function dir(p: string) {
    return p.includes("/") ? p.slice(0, p.lastIndexOf("/")) : "";
  }

  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
  function ago(sec: number): string {
    const d = sec - Date.now() / 1000;
    const steps: [number, Intl.RelativeTimeFormatUnit][] = [
      [60, "second"],
      [60, "minute"],
      [24, "hour"],
      [7, "day"],
      [4.35, "week"],
      [12, "month"],
      [Infinity, "year"],
    ];
    let v = d;
    for (const [n, unit] of steps) {
      if (Math.abs(v) < n) return rtf.format(Math.round(v), unit);
      v /= n;
    }
    return "";
  }

  function toggleCommit(c: GitCommit) {
    openCommit = openCommit === c.hash ? null : c.hash;
    if (openCommit) g.loadCommitFiles(c.hash);
  }

  function letterTitle(l: string): string {
    if (l === "M") return t("git.letter.M");
    if (l === "A") return t("git.letter.A");
    if (l === "D") return t("git.letter.D");
    if (l === "R") return t("git.letter.R");
    if (l === "C") return t("git.letter.C");
    if (l === "U") return t("git.letter.U");
    if (l === "!") return t("git.letter.conflict");
    return t("git.letter.T");
  }

  function busyText(b: string): string {
    if (b === "commit") return t("git.busyCommit");
    if (b === "fetch") return t("git.busyFetch");
    if (b === "pull") return t("git.busyPull");
    if (b === "push") return t("git.busyPush");
    if (b === "switch") return t("git.busySwitch");
    return t("git.busy");
  }

  function commitFileClick(c: GitCommit, files: GitCommitFile[], f: GitCommitFile) {
    g.openCommitDiff(c, files, f);
  }

  /** Arrow keys move between the rows of the list. */
  function onlistkey(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const list = e.currentTarget as HTMLElement;
    const items = [...list.querySelectorAll<HTMLElement>("[data-item]")];
    const i = items.indexOf(document.activeElement as HTMLElement);
    const next = items[e.key === "ArrowDown" ? Math.min(items.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) {
      e.preventDefault();
      next.focus();
    }
  }

  const branchLabel = $derived(st?.branch ?? (st?.head ? t("git.detached", { head: st.head }) : t("git.noBranch")));
</script>

<section class="git" aria-label={t("dock.git")}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="head" onpointerdown={(e) => dock.fromHeader("git", e)}>
    <span class="title">{t("dock.git")}</span>
    <span class="flex-1"></span>
    {#if st?.repo}
      <button class="icon-btn" title={t("git.fetch")} aria-label={t("git.fetch")} disabled={!!g.busy} onclick={() => g.fetch()}><CloudArrowDown size={14} /></button>
      <button class="icon-btn" title={t("git.pull")} aria-label={t("git.pull")} disabled={!!g.busy} onclick={() => g.pull()}><ArrowDown size={14} /></button>
      <button class="icon-btn" title={st.upstream ? t("git.push") : t("git.publish")} aria-label={st.upstream ? t("git.push") : t("git.publish")} disabled={!!g.busy} onclick={() => g.push()}><ArrowUp size={14} /></button>
    {/if}
    <button class="icon-btn" title={t("git.refresh")} aria-label={t("git.refresh")} onclick={() => g.refresh()}><ArrowsClockwise size={13} /></button>
    <DockGrip panel="git" />
    <button class="icon-btn" title={t("common.close")} aria-label={t("dock.close", { panel: t("dock.git") })} onclick={() => dock.close("git")}>
      <X size={13} />
    </button>
  </div>

  {#if g.error && !st}
    <p class="note text-danger selectable">{g.error}</p>
  {:else if !st}
    <p class="note text-fg-muted">{t("common.loading")}</p>
  {:else if !st.repo}
    <div class="empty">
      <p>{t("git.notRepo")}</p>
      <button class="btn btn-primary" data-dock-focus disabled={!!g.busy} onclick={() => g.initRepo()}>{t("git.init")}</button>
      {#if g.actionError}<p class="text-danger selectable">{g.actionError}</p>{/if}
    </div>
  {:else}
    <div class="branch-row">
      <button class="branch" title={t("git.switchHint")} onclick={() => (app.overlay = "branch")}>
        <GitBranch size={14} class="shrink-0" />
        <span class="truncate">{branchLabel}</span>
      </button>
      {#if st.upstream}
        <span class="ab" title={t("git.aheadBehind", { ahead: st.ahead, behind: st.behind, upstream: st.upstream })}>
          <ArrowUp size={11} />{st.ahead}
          <ArrowDown size={11} />{st.behind}
        </span>
      {:else if st.branch}
        <span class="ab" title={t("git.noUpstream")}>{t("git.local")}</span>
      {/if}
      {#if g.busy}<span class="busy" role="status">{busyText(g.busy)}</span>{/if}
    </div>

    <div class="commit">
      <textarea
        bind:this={box}
        data-dock-focus
        rows="2"
        value={message}
        oninput={(e) => (g.messages[project.id] = (e.currentTarget as HTMLTextAreaElement).value)}
        onkeydown={onboxkey}
        placeholder={t("git.messagePlaceholder", { key: keyLabel("Mod+Enter"), branch: st.branch ?? branchLabel })}
        aria-label={t("git.message")}
        spellcheck="true"
      ></textarea>
      <button class="btn btn-primary commit-btn" disabled={!!g.busy || !g.entries.length} onclick={() => g.commit()} title={keyLabel("Mod+Enter")}>
        {nothingStaged ? t("git.commitAll") : t("git.commit")}
      </button>
    </div>

    {#if g.actionError}
      <div class="fail" role="alert">
        <pre class="selectable">{g.actionError}</pre>
        <button class="icon-btn" aria-label={t("common.close")} title={t("common.close")} onclick={() => (g.actionError = null)}><X size={12} /></button>
      </div>
    {/if}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="list" onkeydown={onlistkey}>
      {#if !groups.length}
        <p class="note text-fg-muted">{t("git.clean")}</p>
      {/if}
      {#each groups as grp (grp.id)}
        {@const isFolded = !!folded[grp.id]}
        <div class="group-head">
          <button class="fold" data-item aria-expanded={!isFolded} onclick={() => (folded[grp.id] = !isFolded)}>
            {#if isFolded}<CaretRight size={10} />{:else}<CaretDown size={10} />{/if}
            <span class="truncate">{grp.label}</span>
          </button>
          <span class="tools">
            {#if grp.id === "unstaged" || grp.id === "untracked"}
              <button
                class="icon-btn"
                title={t("git.discardAll")}
                aria-label={t("git.discardAll")}
                disabled={!!g.busy}
                onclick={() => (grp.id === "unstaged" ? g.discard(grp.list, []) : g.discard([], grp.list))}><ArrowCounterClockwise size={13} /></button
              >
            {/if}
            {#if grp.id === "staged"}
              <button class="icon-btn" title={t("git.unstageAll")} aria-label={t("git.unstageAll")} disabled={!!g.busy} onclick={() => g.unstageAll()}><Minus size={13} /></button>
            {:else}
              <button class="icon-btn" title={t("git.stageAll")} aria-label={t("git.stageAll")} disabled={!!g.busy} onclick={() => g.stage(grp.list.map((e) => e.path))}><Plus size={13} /></button>
            {/if}
          </span>
          <span class="n">{grp.list.length}</span>
        </div>
        {#if !isFolded}
          {#each grp.list as e (e.path)}
            {@const letter = letterOf(e, grp.id)}
            <div class="row">
              <button class="file" data-item title={e.orig ? `${e.orig} -> ${e.path}` : e.path} onclick={() => g.openDiff(grp.id, grp.list, e)} ondblclick={() => g.openFile(e)}>
                <FileIcon path={e.path} size={14} />
                <span class="name truncate" class:deleted={letter === "D"}>{name(e.path)}</span>
                <span class="dir truncate">{dir(e.path)}</span>
              </button>
              <span class="tools">
                {#if letter !== "D"}
                  <button class="icon-btn" title={t("git.openFile")} aria-label={t("git.openFile")} onclick={() => g.openFile(e)}><FileIcon2 size={13} /></button>
                {/if}
                {#if grp.id === "unstaged" || grp.id === "untracked"}
                  <button
                    class="icon-btn"
                    title={t("git.discard")}
                    aria-label={t("git.discard")}
                    disabled={!!g.busy}
                    onclick={() => (grp.id === "unstaged" ? g.discard([e], []) : g.discard([], [e]))}><ArrowCounterClockwise size={13} /></button
                  >
                {/if}
                {#if grp.id === "staged"}
                  <button class="icon-btn" title={t("git.unstage")} aria-label={t("git.unstage")} disabled={!!g.busy} onclick={() => g.unstage([e.path])}><Minus size={13} /></button>
                {:else}
                  <button class="icon-btn" title={t("git.stage")} aria-label={t("git.stage")} disabled={!!g.busy} onclick={() => g.stage([e.path])}><Plus size={13} /></button>
                {/if}
              </span>
              <span class="letter l-{letter === '!' ? 'conflict' : letter}" title={letterTitle(letter)}>{letter}</span>
            </div>
          {/each}
        {/if}
      {/each}
      {#if st.truncated}
        <p class="note text-fg-muted">{t("git.truncated")}</p>
      {/if}

      <div class="group-head history">
        <button class="fold" data-item aria-expanded={!folded.history} onclick={() => (folded.history = !folded.history)}>
          {#if folded.history}<CaretRight size={10} />{:else}<CaretDown size={10} />{/if}
          <span class="truncate">{t("git.history")}</span>
        </button>
      </div>
      {#if !folded.history}
        {#if !g.log.length}
          <p class="note text-fg-muted">{t("git.noCommits")}</p>
        {/if}
        {#each g.log as c (c.hash)}
          <button class="commit-row" data-item aria-expanded={openCommit === c.hash} title="{c.short} {c.subject}" onclick={() => toggleCommit(c)}>
            <span class="subject truncate">{c.subject}</span>
            <span class="meta truncate">{c.author}, {ago(c.time)}</span>
          </button>
          {#if openCommit === c.hash}
            {@const files = g.commitFiles[c.hash]}
            {#if !files}
              <p class="note sub text-fg-muted">{t("common.loading")}</p>
            {:else if !files.length}
              <p class="note sub text-fg-muted">{t("git.noFiles")}</p>
            {:else}
              {#each files as f (f.path)}
                <button class="file sub" data-item title={f.orig ? `${f.orig} -> ${f.path}` : f.path} onclick={() => commitFileClick(c, files, f)}>
                  <FileIcon path={f.path} size={14} />
                  <span class="name truncate" class:deleted={f.status === "D"}>{name(f.path)}</span>
                  <span class="dir truncate">{dir(f.path)}</span>
                  <span class="letter l-{f.status}" title={letterTitle(f.status)}>{f.status}</span>
                </button>
              {/each}
            {/if}
          {/if}
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .git {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-subtle);
    min-height: 0;
  }
  .head {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  .note {
    padding: 6px 12px;
    font-size: 12px;
  }
  .note.sub {
    padding-inline-start: 28px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 14px 12px;
    color: var(--fg-muted);
  }
  .branch-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 2px;
    min-width: 0;
    flex-shrink: 0;
  }
  .branch {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    font-weight: 500;
  }
  .branch:hover {
    background: var(--bg-hover);
  }
  .ab {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
    font-size: 11.5px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
  .busy {
    margin-inline-start: auto;
    flex-shrink: 0;
    font-size: 11.5px;
    color: var(--fg-muted);
  }
  .commit {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 8px 8px;
    flex-shrink: 0;
  }
  .commit textarea {
    width: 100%;
    min-height: 44px;
    max-height: 200px;
    resize: vertical;
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    outline: none;
    user-select: text;
  }
  .commit textarea:focus {
    border-color: var(--fg-faint);
  }
  .commit-btn {
    justify-content: center;
    width: 100%;
  }
  .fail {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0 8px 6px;
    padding: 6px 4px 6px 10px;
    border-radius: 6px;
    background: var(--danger-soft);
    color: var(--danger);
    flex-shrink: 0;
  }
  .fail pre {
    flex: 1;
    min-width: 0;
    max-height: 160px;
    overflow: auto;
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 12px;
    border-top: 1px solid var(--border);
  }
  .group-head,
  .row {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    height: 24px;
    padding-inline-end: 8px;
  }
  .group-head:hover,
  .row:hover {
    background: var(--bg-hover);
  }
  .group-head.history {
    margin-top: 8px;
    border-top: 1px solid var(--border);
    height: 28px;
  }
  .fold {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 6px 0 8px;
    border: 0;
    background: transparent;
    text-align: start;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }
  .tools {
    display: none;
    align-items: center;
    flex-shrink: 0;
  }
  .group-head:hover .tools,
  .row:hover .tools,
  .group-head:focus-within .tools,
  .row:focus-within .tools {
    display: inline-flex;
  }
  .tools :global(.icon-btn) {
    width: 22px;
    height: 22px;
  }
  .n {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--bg-active);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .file,
  .commit-row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 6px 0 22px;
    border: 0;
    background: transparent;
    text-align: start;
    white-space: nowrap;
  }
  .file.sub {
    width: 100%;
    padding-inline-start: 28px;
    padding-inline-end: 8px;
  }
  .file.sub:hover,
  .commit-row:hover {
    background: var(--bg-hover);
  }
  .file:focus-visible,
  .fold:focus-visible,
  .commit-row:focus-visible {
    outline-offset: -2px;
  }
  .name {
    min-width: 0;
    flex-shrink: 0;
    max-width: 65%;
  }
  .name.deleted {
    text-decoration: line-through;
    color: var(--fg-muted);
  }
  .dir {
    min-width: 0;
    font-size: 11.5px;
    color: var(--fg-faint);
  }
  .letter {
    width: 14px;
    flex-shrink: 0;
    margin-inline-start: auto;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 600;
  }
  .l-M,
  .l-T,
  .l-R,
  .l-C {
    color: var(--warn);
  }
  .l-A,
  .l-U {
    color: var(--add);
  }
  .l-D,
  .l-conflict {
    color: var(--del);
  }
  .commit-row {
    width: 100%;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    gap: 0;
    height: 38px;
    padding: 0 10px 0 12px;
  }
  .subject {
    min-width: 0;
  }
  .meta {
    min-width: 0;
    font-size: 11.5px;
    color: var(--fg-faint);
  }
</style>
