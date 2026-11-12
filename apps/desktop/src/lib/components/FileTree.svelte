<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { ArrowsLeftRight, CaretDown, CaretRight, PushPin } from "phosphor-svelte";
  import { t } from "../i18n";
  import { invalidate } from "../files.svelte";
  import { app, type Project } from "../stores/app.svelte";
  import FileIcon from "./FileIcon.svelte";

  interface Node {
    name: string;
    path: string;
    dir: boolean;
    ignored: boolean;
  }

  let { project }: { project: Project } = $props();

  let children = $state<Record<string, Node[]>>({});
  let expanded = $state<Record<string, boolean>>({});
  let error = $state<string | null>(null);
  let cursor = $state<string | null>(null);
  let tree: HTMLDivElement | undefined = $state();

  async function load(dir: string) {
    try {
      const list = await invoke<Node[]>("fs_list_dir", {
        projectId: project.id,
        path: dir,
        showIgnored: app.settings.showIgnored,
      });
      children[dir] = list;
      if (dir === "") error = null;
    } catch (e) {
      if (dir === "") error = String(e);
    }
  }

  // Reload when the project or the "show ignored files" setting changes.
  $effect(() => {
    void project.id;
    void app.settings.showIgnored;
    untrack(() => {
      children = {};
      load("");
      for (const d of Object.keys(expanded)) if (expanded[d]) load(d);
    });
  });

  // Folder changes: reload the folders that changed and are loaded.
  $effect(() => {
    const id = project.id;
    const un = listen<{ projectId: string; paths: string[] }>("fs-changed", (e) => {
      if (e.payload.projectId !== id) return;
      invalidate(id);
      const dirs = new Set<string>();
      for (const p of e.payload.paths) {
        const i = p.lastIndexOf("/");
        dirs.add(i < 0 ? "" : p.slice(0, i));
      }
      for (const d of dirs) if (d in children) load(d);
    });
    return () => {
      un.then((f) => f());
    };
  });

  const pinned = $derived((children[""] ?? []).find((n) => !n.dir && n.name.toLowerCase() === "claude.md"));

  /** Visible rows in order, for keyboard navigation. */
  const rows = $derived.by(() => {
    const out: { node: Node; depth: number }[] = [];
    const walk = (dir: string, depth: number) => {
      for (const n of children[dir] ?? []) {
        out.push({ node: n, depth });
        if (n.dir && expanded[n.path]) walk(n.path, depth + 1);
      }
    };
    walk("", 0);
    return out;
  });

  function toggle(n: Node) {
    expanded[n.path] = !expanded[n.path];
    if (expanded[n.path] && !children[n.path]) load(n.path);
  }

  function open(n: Node) {
    cursor = n.path;
    if (n.dir) toggle(n);
    else app.viewer = { kind: "file", projectId: project.id, path: n.path };
  }

  function onkeydown(e: KeyboardEvent) {
    const i = rows.findIndex((r) => r.node.path === cursor);
    const cur = rows[i]?.node;
    if (e.key === "ArrowDown") cursor = rows[Math.min(rows.length - 1, i + 1)]?.node.path ?? cursor;
    else if (e.key === "ArrowUp") cursor = rows[Math.max(0, i - 1)]?.node.path ?? cursor;
    else if (e.key === "ArrowRight" && cur?.dir) {
      if (!expanded[cur.path]) toggle(cur);
      else cursor = rows[i + 1]?.node.path ?? cursor;
    } else if (e.key === "ArrowLeft" && cur) {
      if (cur.dir && expanded[cur.path]) toggle(cur);
      else {
        const parent = cur.path.includes("/") ? cur.path.slice(0, cur.path.lastIndexOf("/")) : null;
        if (parent) cursor = parent;
      }
    } else if (e.key === "Enter" && cur) open(cur);
    else return;
    e.preventDefault();
    queueMicrotask(() => tree?.querySelector(`[data-path="${CSS.escape(cursor ?? "")}"]`)?.scrollIntoView({ block: "nearest" }));
  }

  const viewing = $derived(app.viewer?.kind === "file" ? app.viewer.path : null);
</script>

<aside class="files">
  <div class="head">
    <span class="flex-1">{t("files.title")}</span>
    <button
      class="icon-btn move"
      onclick={() => app.movePanel("files")}
      title={app.layout.filesSide === "left" ? t("layout.moveRight") : t("layout.moveLeft")}
      aria-label={app.layout.filesSide === "left" ? t("layout.moveRight") : t("layout.moveLeft")}
    >
      <ArrowsLeftRight size={12} />
    </button>
  </div>
  {#if pinned}
    <button class="row pinned" class:on={viewing === pinned.path} onclick={() => open(pinned)}>
      <PushPin size={12} class="shrink-0 text-fg-faint" />
      <FileIcon path={pinned.name} size={15} />
      <span class="truncate">{pinned.name}</span>
    </button>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="tree" bind:this={tree} tabindex="0" role="tree" aria-label={t("files.title")} {onkeydown}>
    {#if error}
      <p class="px-3 py-2 text-danger">{t("files.error", { error })}</p>
    {:else if children[""] && !children[""].length}
      <p class="px-3 py-2 text-fg-muted">{t("files.empty")}</p>
    {/if}
    {#each rows as { node, depth } (node.path)}
      <button
        class="row"
        class:on={viewing === node.path}
        class:cursor={cursor === node.path}
        class:ignored={node.ignored}
        style="padding-inline-start: {8 + depth * 14}px"
        data-path={node.path}
        role="treeitem"
        aria-expanded={node.dir ? !!expanded[node.path] : undefined}
        aria-selected={viewing === node.path}
        tabindex="-1"
        onclick={() => open(node)}
        title={node.path}
      >
        <span class="caret">
          {#if node.dir}
            {#if expanded[node.path]}<CaretDown size={10} />{:else}<CaretRight size={10} />{/if}
          {/if}
        </span>
        <FileIcon path={node.name} folder={node.dir} open={!!expanded[node.path]} size={15} />
        <span class="truncate">{node.name}</span>
      </button>
    {/each}
  </div>
</aside>

<style>
  .move {
    opacity: 0;
  }
  .files:hover .move,
  .move:focus-visible {
    opacity: 1;
  }
  .files {
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
    padding: 0 6px 0 12px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tree {
    flex: 1;
    overflow: auto;
    padding: 4px 0 12px;
    outline: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 26px;
    padding-inline-end: 8px;
    border: 0;
    background: transparent;
    text-align: start;
    white-space: nowrap;
    min-width: 0;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.on {
    background: var(--bg-active);
  }
  .tree:focus-visible .row.cursor {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .row.ignored {
    color: var(--fg-faint);
  }
  .pinned {
    padding-inline-start: 10px;
    border-bottom: 1px solid var(--border);
    height: 30px;
    flex-shrink: 0;
  }
  .caret {
    width: 10px;
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--fg-faint);
  }
</style>
