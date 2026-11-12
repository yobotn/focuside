<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { highlightIn, renderMarkdown } from "../markdown";

  let { text, streaming = false, class: cls = "" }: { text: string; streaming?: boolean; class?: string } = $props();

  let el: HTMLDivElement | undefined = $state();
  let html = $state("");
  let frame = 0;

  // Re-render at most once per frame while text streams in.
  $effect(() => {
    const value = text;
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      html = renderMarkdown(value);
    });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    void html;
    if (!streaming && el) {
      const target = el;
      queueMicrotask(() => highlightIn(target));
    }
  });

  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (/^https?:\/\//.test(href)) openUrl(href).catch(() => {});
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={el} class="doc selectable {cls}" {onclick}>{@html html}</div>
