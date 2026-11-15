<script lang="ts">
  // The path of the open file above the editor: folder > folder > file. A click on a part
  // shows it in the file tree.
  import { CaretRight } from "phosphor-svelte";
  import { t } from "../i18n";
  import { editor } from "../stores/editor.svelte";
  import FileIcon from "./FileIcon.svelte";

  let { projectId, path }: { projectId: string; path: string } = $props();

  const parts = $derived(
    path.split("/").map((name, i, all) => ({ name, path: all.slice(0, i + 1).join("/"), file: i === all.length - 1 })),
  );
</script>

<nav class="crumbs" aria-label={t("editor.breadcrumbs")}>
  <ol>
    {#each parts as p, i (p.path)}
      <li>
        {#if i > 0}<CaretRight size={10} class="sep" aria-hidden="true" />{/if}
        <button
          class="part"
          class:file={p.file}
          title={t("editor.revealPart", { path: p.path })}
          aria-current={p.file ? "page" : undefined}
          onclick={() => editor.requestReveal(projectId, p.path)}
        >
          {#if p.file}<FileIcon path={p.name} size={13} />{/if}
          <span class="truncate">{p.name}</span>
        </button>
      </li>
    {/each}
  </ol>
</nav>

<style>
  .crumbs {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
  }
  ol {
    display: flex;
    align-items: center;
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    min-width: 0;
    flex-shrink: 1;
  }
  li:last-child {
    flex-shrink: 0;
  }
  li :global(.sep) {
    flex-shrink: 0;
    margin: 0 1px;
    color: var(--fg-faint);
  }
  .part {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    height: 20px;
    padding: 0 4px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  .part:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .part.file {
    color: var(--fg);
  }
</style>
