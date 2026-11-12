<script lang="ts">
  import { FolderOpen } from "phosphor-svelte";
  import { t } from "../i18n";

  let { recent = [], onopen, onopenpath }: {
    recent?: { path: string; name: string }[];
    onopen: () => void;
    onopenpath: (path: string) => void;
  } = $props();
</script>

<div class="grid h-full place-items-center bg-bg">
  <div class="w-[420px] max-w-[90vw]">
    <h1 class="mb-2 text-[20px] font-semibold tracking-tight">{t("empty.title")}</h1>
    <p class="mb-6 font-read text-[15px] leading-relaxed text-fg-muted">{t("empty.body")}</p>
    <button class="btn btn-primary h-9 px-4" onclick={onopen}>
      <FolderOpen size={16} />
      {t("empty.open")}
    </button>
    <p class="mt-3 text-fg-muted">{t("empty.drop")} {t("empty.cli")}</p>

    {#if recent.length}
      <h2 class="mt-8 mb-2 text-[11px] font-semibold tracking-wide text-fg-muted uppercase">{t("empty.recent")}</h2>
      <ul class="-mx-2">
        {#each recent as r (r.path)}
          <li>
            <button class="flex h-7 w-full items-center gap-3 rounded-md px-2 text-start hover:bg-bg-hover" onclick={() => onopenpath(r.path)}>
              <span class="font-medium">{r.name}</span>
              <span class="truncate text-fg-muted">{r.path}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
