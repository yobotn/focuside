// Markdown for the agent's text and .md files. Agent output is untrusted: everything goes
// through DOMPurify. Code blocks are highlighted by Shiki, loaded lazily with only the
// grammars a document uses.
import MarkdownIt from "markdown-it";
import DOMPurify from "dompurify";
import type { HighlighterCore } from "shiki/core";

const md = new MarkdownIt({ html: false, linkify: true, breaks: false, typographer: false });

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
