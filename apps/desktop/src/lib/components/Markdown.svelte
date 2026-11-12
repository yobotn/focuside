<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { highlightIn, renderDocument, renderMarkdown } from "../markdown";

  let {
    text,
    streaming = false,
    class: cls = "",
    doc,
  }: {
    text: string;
    streaming?: boolean;
    class?: string;
    /** A markdown file in a project: embedded HTML, local images and links between files. */
    doc?: { projectId: string; path: string; open: (path: string) => void };
  } = $props();

  let el: HTMLDivElement | undefined = $state();
  let html = $state("");
  let frame = 0;

  // Re-render at most once per frame while text streams in.
  $effect(() => {
    const value = text;
    const isDoc = !!doc;
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      html = isDoc ? renderDocument(value) : renderMarkdown(value);
    });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    void html;
    if (!streaming && el) {
      const target = el;
      queueMicrotask(() => {
        highlightIn(target);
        loadImages(target);
      });
    }
  });

  /** Joins a link inside the document with the document's folder. Null when it leaves the project. */
  function resolve(href: string): string | null {
    const base = doc?.path.split("/").slice(0, -1) ?? [];
    const parts = href.startsWith("/") ? [] : [...base];
    for (const seg of decodeURIComponent(href.replace(/[?#].*$/, "")).split("/")) {
      if (!seg || seg === ".") continue;
      if (seg === "..") {
        if (!parts.length) return null;
        parts.pop();
      } else parts.push(seg);
    }
    return parts.join("/");
  }

  const external = /^[a-z][a-z0-9+.-]*:/i;

  // Images in the project load through the app; images on the web are not fetched (privacy and
  // the content policy), so they show as a link with their alt text.
  function loadImages(root: HTMLElement) {
    for (const img of Array.from(root.querySelectorAll<HTMLImageElement>("img"))) {
      if (img.dataset.done) continue;
      img.dataset.done = "1";
      const src = img.getAttribute("src") ?? "";
      if (src.startsWith("data:")) continue;
      if (/^https?:/i.test(src)) {
        const a = document.createElement("a");
        a.href = img.closest("a")?.getAttribute("href") ?? src;
        a.className = "img-remote";
        a.textContent = img.alt || src.split("/").pop() || src;
        a.title = src;
        (img.closest("a") ?? img).replaceWith(a);
        continue;
      }
      const path = doc && !external.test(src) ? resolve(src) : null;
      if (!doc || !path) {
        img.remove();
        continue;
      }
      img.removeAttribute("src");
      invoke<string>("fs_image", { projectId: doc.projectId, path })
        .then((url) => (img.src = url))
        .catch(() => {
          const s = document.createElement("span");
          s.className = "img-remote";
          s.textContent = img.alt || path;
          img.replaceWith(s);
        });
    }
  }

  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (/^(https?|mailto):/i.test(href)) {
      openUrl(href).catch(() => {});
    } else if (href.startsWith("#")) {
      const id = decodeURIComponent(href.slice(1));
      el?.querySelector(`[id="${CSS.escape(id)}"]`)?.scrollIntoView({ block: "start", behavior: "smooth" });
    } else if (doc && href && !external.test(href)) {
      const path = resolve(href);
      if (path) doc.open(path);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={el} class="doc selectable {cls}" class:document={!!doc} {onclick}>{@html html}</div>
