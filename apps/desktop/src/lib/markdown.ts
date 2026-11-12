// Markdown for the agent's text and .md files. Agent output is untrusted: everything goes
// through DOMPurify. Code blocks are highlighted by Shiki, loaded lazily with only the
// grammars a document uses.
import MarkdownIt from "markdown-it";
import DOMPurify from "dompurify";
import type { HighlighterCore } from "shiki/core";

const md = new MarkdownIt({ html: false, linkify: true, breaks: false, typographer: false });
// Files in the project are documents: embedded HTML (READMEs use it a lot) renders, sanitized.
const docMd = new MarkdownIt({ html: true, linkify: true, breaks: false, typographer: false });

for (const m of [md, docMd]) {
  m.core.ruler.push("focuside_extras", (state) => {
    const tokens = state.tokens;
    const slugs = new Map<string, number>();
    for (let i = 0; i < tokens.length; i++) {
      const tok = tokens[i];
      // Heading ids, GitHub style, so in-document links work.
      if (tok.type === "heading_open") {
        const text = tokens[i + 1]?.children?.map((c) => c.content).join("") ?? "";
        let slug = text
          .toLowerCase()
          .trim()
          .replace(/[^\p{L}\p{N}\s_-]/gu, "")
          .replace(/\s/g, "-");
        const n = slugs.get(slug) ?? 0;
        slugs.set(slug, n + 1);
        if (n) slug += `-${n}`;
        tok.attrSet("id", slug);
      }
      // Task list items: "- [ ] x" and "- [x] x".
      if (tok.type === "inline" && tokens[i - 1]?.type === "paragraph_open" && tokens[i - 2]?.type === "list_item_open") {
        const first = tok.children?.[0];
        const mm = first?.type === "text" ? first.content.match(/^\[([ xX])\]\s/) : null;
        if (first && mm) {
          first.content = first.content.slice(mm[0].length);
          const box = new state.Token("html_inline", "", 0);
          box.content = `<input type="checkbox" disabled${mm[1] === " " ? "" : " checked"}>`;
          tok.children!.unshift(box);
          tokens[i - 2].attrJoin("class", "task");
        }
      }
      // GitHub alerts: "> [!NOTE]" and friends.
      if (tok.type === "blockquote_open" && tokens[i + 1]?.type === "paragraph_open") {
        const inline = tokens[i + 2];
        const first = inline?.children?.[0];
        const mm = first?.type === "text" ? first.content.match(/^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*$/i) : null;
        if (inline && first && mm) {
          const kind = mm[1].toLowerCase();
          tok.attrJoin("class", `alert alert-${kind}`);
          first.content = kind[0].toUpperCase() + kind.slice(1);
          const next = inline.children![1];
          if (next?.type === "softbreak") inline.children!.splice(1, 1);
          const strongOpen = new state.Token("html_inline", "", 0);
          strongOpen.content = '<span class="alert-title">';
          const strongClose = new state.Token("html_inline", "", 0);
          strongClose.content = "</span>";
          inline.children!.splice(0, 1, strongOpen, first, strongClose);
        }
      }
    }
  });
}

// Inputs (task-list checkboxes, or HTML in a document) are always read-only.
DOMPurify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "INPUT") node.setAttribute("disabled", "");
});

// Links open in the system browser (handled by a click listener), never inside the app.
DOMPurify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "A") {
    node.setAttribute("rel", "noopener noreferrer");
    node.setAttribute("target", "_blank");
  }
});

let highlighter: HighlighterCore | null = null;
let loading: Promise<HighlighterCore> | null = null;
const loadedLangs = new Set<string>();

const aliases: Record<string, string> = {
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  ts: "typescript",
  rs: "rust",
  py: "python",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  ps1: "powershell",
  pwsh: "powershell",
  yml: "yaml",
  md: "markdown",
  "c++": "cpp",
  cs: "csharp",
  rb: "ruby",
  kt: "kotlin",
  golang: "go",
  htm: "html",
  jsonc: "json",
};

async function getHighlighter(): Promise<HighlighterCore> {
  if (highlighter) return highlighter;
  if (!loading) {
    loading = (async () => {
      const [{ createHighlighterCore }, { createJavaScriptRegexEngine }, light, dark] = await Promise.all([
        import("shiki/core"),
        import("shiki/engine/javascript"),
        import("shiki/themes/github-light-default.mjs"),
        import("shiki/themes/github-dark-default.mjs"),
      ]);
      highlighter = await createHighlighterCore({
        themes: [light.default, dark.default],
        langs: [],
        engine: createJavaScriptRegexEngine(),
      });
      return highlighter;
    })();
  }
  return loading;
}

async function ensureLang(h: HighlighterCore, lang: string): Promise<boolean> {
  if (loadedLangs.has(lang)) return true;
  try {
    const { bundledLanguages } = await import("shiki/langs");
    const loader = (bundledLanguages as Record<string, () => Promise<any>>)[lang];
    if (!loader) return false;
    await h.loadLanguage(await loader());
    loadedLangs.add(lang);
    return true;
  } catch {
    return false;
  }
}

/** Markdown to sanitized HTML without highlighting (fast, for streaming text). */
export function renderMarkdown(text: string): string {
  return DOMPurify.sanitize(md.render(text));
}

/** Splits YAML front matter off a document. */
function frontMatter(text: string): { meta: [string, string][] | null; body: string } {
  const m = text.match(/^﻿?---\r?\n([\s\S]*?)\r?\n(?:---|\.\.\.)[ \t]*(?:\r?\n|$)/);
  if (!m) return { meta: null, body: text };
  const meta: [string, string][] = [];
  let last: [string, string] | null = null;
  for (const line of m[1].split(/\r?\n/)) {
    const kv = line.match(/^([\w.-]+):\s*(.*)$/);
    if (kv) {
      last = [kv[1], kv[2].replace(/^["']|["']$/g, "")];
      meta.push(last);
    } else if (last && line.trim()) {
      last[1] += (last[1] ? " " : "") + line.trim().replace(/^-\s*/, "");
    }
  }
  return { meta, body: text.slice(m[0].length) };
}

function escapeHtml(s: string) {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);
}

/** A markdown file as a document: front matter as a table, embedded HTML sanitized. */
export function renderDocument(text: string): string {
  const { meta, body } = frontMatter(text);
  let head = "";
  if (meta?.length) {
    head =
      '<table class="front-matter"><tbody>' +
      meta.map(([k, v]) => `<tr><th>${escapeHtml(k)}</th><td>${escapeHtml(v)}</td></tr>`).join("") +
      "</tbody></table>";
  }
  return DOMPurify.sanitize(head + docMd.render(body), {
    ADD_TAGS: ["details", "summary"],
    ADD_ATTR: ["align", "open"],
    FORBID_TAGS: ["style", "form", "button", "textarea", "select", "iframe"],
  });
}

/** Highlights the code blocks inside an element rendered by `renderMarkdown`. */
export async function highlightIn(el: HTMLElement) {
  const blocks = Array.from(el.querySelectorAll<HTMLElement>("pre > code[class*='language-']"));
  if (!blocks.length) return;
  const h = await getHighlighter();
  for (const code of blocks) {
    if (!code.isConnected || code.dataset.hl) continue;
    const m = code.className.match(/language-([\w+#-]+)/);
    if (!m) continue;
    const lang = aliases[m[1].toLowerCase()] ?? m[1].toLowerCase();
    if (!(await ensureLang(h, lang))) continue;
    const html = h.codeToHtml(code.textContent ?? "", {
      lang,
      themes: { light: "github-light-default", dark: "github-dark-default" },
      defaultColor: false,
    });
    const pre = code.parentElement;
    if (!pre?.isConnected) continue;
    const tmp = document.createElement("div");
    tmp.innerHTML = DOMPurify.sanitize(html);
    const fresh = tmp.firstElementChild as HTMLElement | null;
    if (fresh) {
      fresh.querySelector("code")?.setAttribute("data-hl", "1");
      pre.replaceWith(fresh);
    }
  }
}

/** Highlights a whole source text (for the markdown source view and small files). */
export async function highlightCode(text: string, lang: string): Promise<string | null> {
  const h = await getHighlighter();
  const l = aliases[lang] ?? lang;
  if (!(await ensureLang(h, l))) return null;
  return DOMPurify.sanitize(
    h.codeToHtml(text, {
      lang: l,
      themes: { light: "github-light-default", dark: "github-dark-default" },
      defaultColor: false,
    }),
  );
}
