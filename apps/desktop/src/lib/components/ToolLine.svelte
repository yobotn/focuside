<script lang="ts">
  import { Eye, FilePlus, Globe, MagnifyingGlass, PencilSimple, Terminal, Trash, ArrowsLeftRight, Wrench, Brain } from "phosphor-svelte";
  import { t } from "../i18n";
  import type { Entry } from "../types";
  import { cleanTitle, relTo, type Session } from "../stores/session.svelte";
  import { app } from "../stores/app.svelte";

  let { session, entry }: { session: Session; entry: Extract<Entry, { t: "tool" }> } = $props();

  const Icon = $derived(
    {
      read: Eye,
      edit: PencilSimple,
      delete: Trash,
      move: ArrowsLeftRight,
      search: MagnifyingGlass,
      execute: Terminal,
      fetch: Globe,
      think: Brain,
    }[entry.kind] ?? (entry.title.toLowerCase().startsWith("write") ? FilePlus : Wrench),
  );

  // Paths in titles come from the agent with OS separators; show them relative and with "/".
  const title = $derived(cleanTitle(session.root, entry.title || entry.kind, entry.paths));
  const file = $derived.by(() => {
    const p = entry.paths[0];
    if (!p) return null;
    return relTo(session.root, p);
  });
  const failed = $derived(entry.status === "failed");
  const active = $derived(entry.status === "pending" || entry.status === "in_progress");

  function open() {
    if (file && entry.kind !== "delete") app.viewer = { kind: "file", projectId: session.projectId, path: file };
  }
</script>

<div class="line" class:active>
  <Icon size={14} class="shrink-0 text-fg-faint" />
  {#if file && entry.kind !== "execute"}
    <button class="truncate text-start hover:underline" onclick={open} title={title}>{title}</button>
  {:else}
    <span class="truncate selectable" class:mono={entry.kind === "execute"} title={title}>{title}</span>
  {/if}
  {#if failed}<span class="shrink-0 text-danger">{t("conv.toolFailed")}</span>{/if}
</div>

<style>
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    color: var(--fg-muted);
    min-width: 0;
  }
  .line button {
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    min-width: 0;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
