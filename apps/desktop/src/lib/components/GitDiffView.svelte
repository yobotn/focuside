<script lang="ts">
  // A git change of the project's own repository (Git panel): one file at a time, old and new
  // side in CodeMirror. Turn changes use ReviewView instead. J and K step through the files.
  import { untrack } from "svelte";
  import { CaretLeft, CaretRight } from "phosphor-svelte";
  import { api, errText } from "../api";
  import { t } from "../i18n";
  import { app, type Project } from "../stores/app.svelte";
  import { editor, type DiffTab, type Workspace } from "../stores/editor.svelte";
  import { git as gitStore, gitDiffName } from "../stores/git.svelte";
  import type { FileVersions } from "../types";
  import CodeEditor from "./CodeEditor.svelte";

  let { project, w, tab }: { project: Project; w: Workspace; tab: DiffTab } = $props();

  const path = $derived(tab.paths[tab.index] ?? "");
  let versions = $state<FileVersions | null>(null);
  let error = $state<string | null>(null);

  async function load(p: string, orig: string | null, quiet = false) {
    const g = tab.git;
    if (!g) return;
    if (!quiet) {
      error = null;
      versions = null;
    }
    try {
      const r = await api.gitVersions(project.id, g.source, p, orig);
      if (p === path && JSON.stringify(r) !== JSON.stringify(versions)) versions = r;
    } catch (e) {
      if (p === path) error = errText(e);
    }
  }

  $effect(() => {
    const p = path;
    const orig = tab.git?.origs[tab.index] ?? null;
    untrack(() => load(p, orig));
  });

  // A diff of the work tree or the index follows the repository, and reloads in place.
  let lastGit = -1;
  $effect(() => {
    const v = gitStore.version;
    untrack(() => {
      if (lastGit >= 0 && v !== lastGit && tab.git && tab.git.source.kind !== "commit") load(path, tab.git.origs[tab.index] ?? null, true);
    });
    lastGit = v;
  });

  function step(d: number) {
    editor.stepDiff(w, tab.index + d);
  }

  function onkeydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    const typing = !!el && (el.tagName === "TEXTAREA" || el.tagName === "INPUT" || el.isContentEditable);
    if (typing || e.ctrlKey || e.metaKey || e.altKey || app.overlay || app.undoTarget) return;
    if (e.key === "j" || e.key === "J") step(1);
    else if (e.key === "k" || e.key === "K") step(-1);
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="gitdiff" {onkeydown}>
  <div class="bar">
    <span class="text-fg-muted">{tab.git ? gitDiffName(tab.git) : ""}</span>
    <span class="truncate font-medium" title={path}>{path}</span>
    <span class="flex-1"></span>
    <span class="text-fg-muted">{t("viewer.diffOf", { index: tab.index + 1, count: tab.paths.length })}</span>
    <button class="icon-btn" onclick={() => step(-1)} disabled={tab.index === 0} title="{t('viewer.prevFile')} (K)" aria-label={t("viewer.prevFile")}><CaretLeft size={14} /></button>
    <button class="icon-btn" onclick={() => step(1)} disabled={tab.index >= tab.paths.length - 1} title="{t('viewer.nextFile')} (J)" aria-label={t("viewer.nextFile")}><CaretRight size={14} /></button>
  </div>
  {#if error}
    <p class="p-4 text-danger selectable">{error}</p>
  {:else if !versions}
    <p class="p-4 text-fg-muted">{t("common.loading")}</p>
  {:else if versions.binary}
    <p class="p-4 text-fg-muted">{t("viewer.binary", { size: "" })}</p>
  {:else if versions.after === null}
    <p class="px-4 pt-3 text-fg-muted">{t("viewer.gitDeleted")}</p>
    <CodeEditor {path} text={versions.before ?? ""} readonly />
  {:else}
    <CodeEditor {path} text={versions.after} original={versions.before ?? ""} readonly />
  {/if}
</div>

<style>
  .gitdiff {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-width: 0;
  }
</style>
