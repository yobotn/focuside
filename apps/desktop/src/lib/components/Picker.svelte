<script lang="ts" module>
  export interface PickerItem {
    value: string;
    label: string;
    detail?: string;
  }
</script>

<script lang="ts">
  import { t } from "../i18n";
  import { allFiles } from "../files.svelte";
  import { filter } from "../fuzzy";
  import type { Session } from "../stores/session.svelte";
  import FileIcon from "./FileIcon.svelte";

  let {
    kind,
    query,
    session,
    onpick,
    onclose,
  }: {
    kind: "files" | "commands";
    query: string;
    session: Session;
    onpick: (item: PickerItem) => void;
    onclose: () => void;
  } = $props();

  let files = $state<string[]>([]);
  let index = $state(0);
  let list: HTMLUListElement | undefined = $state();

  $effect(() => {
    if (kind === "files") allFiles(session.projectId).then((f) => (files = f)).catch(() => {});
  });

  const items = $derived.by((): PickerItem[] => {
    if (kind === "files") return filter(files, query, (f) => f, 12).map((f) => ({ value: f, label: f }));
    return filter(session.commands, query, (c) => c.name, 30).map((c) => ({
      value: c.name,
      label: "/" + c.name,
      detail: c.description,
    }));
  });

  $effect(() => {
    void query;
    index = 0;
  });

  $effect(() => {
    list?.children[index]?.scrollIntoView({ block: "nearest" });
  });

  /** Handles a key from the message box. Returns true when the picker used it. */
  export function key(e: KeyboardEvent): boolean {
    if (e.key === "ArrowDown") index = Math.min(items.length - 1, index + 1);
    else if (e.key === "ArrowUp") index = Math.max(0, index - 1);
    else if ((e.key === "Enter" || e.key === "Tab") && items[index] && !e.shiftKey) onpick(items[index]);
    else if (e.key === "Escape") onclose();
    else return false;
    e.preventDefault();
    e.stopPropagation();
    return true;
  }
</script>

<div class="picker" role="listbox">
  {#if items.length === 0}
    <p class="px-3 py-2 text-fg-muted">{kind === "files" ? t("box.noFiles") : t("box.noCommands")}</p>
  {:else}
    <ul bind:this={list}>
      {#each items as item, i (item.value)}
        <li>
          <button
            class="item"
            class:active={i === index}
            onmousedown={(e) => {
              e.preventDefault();
              onpick(item);
            }}
            onmousemove={() => (index = i)}
          >
            {#if kind === "files"}<FileIcon path={item.value} size={14} />{/if}
            <span class="truncate" class:font-mono={kind === "files"}>{item.label}</span>
            {#if item.detail}<span class="ms-2 truncate text-fg-muted">{item.detail}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .picker {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    right: 0;
    max-height: 300px;
    overflow-y: auto;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
    z-index: 20;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: var(--row);
    padding: 0 8px;
    border-radius: 5px;
    border: 0;
    background: transparent;
    text-align: start;
    min-width: 0;
  }
  .item.active {
    background: var(--bg-hover);
  }
  .font-mono {
    font-size: 12.5px;
  }
</style>
