// CodeMirror setup shared by the viewer and the diff viewer: theme from the app's color
// tokens (light and dark follow the CSS variables), language from the file name.
import { EditorView } from "@codemirror/view";
import { HighlightStyle, LanguageDescription, syntaxHighlighting } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { tags } from "@lezer/highlight";
import { EditorState, type Extension } from "@codemirror/state";
import { t } from "./i18n";

export const theme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "13px",
    backgroundColor: "var(--bg)",
    color: "var(--fg)",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.55",
  },
  ".cm-content": { caretColor: "var(--fg)", padding: "8px 0" },
  ".cm-cursor": { borderLeftColor: "var(--fg)" },
  ".cm-gutters": {
    backgroundColor: "var(--bg)",
    color: "var(--fg-faint)",
    border: "none",
  },
  ".cm-activeLine": { backgroundColor: "color-mix(in srgb, var(--fg) 4%, transparent)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--fg-muted)" },
  "&.cm-focused": { outline: "none" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "color-mix(in srgb, var(--project) 28%, transparent) !important",
  },
  ".cm-searchMatch": { backgroundColor: "var(--warn-soft)", outline: "1px solid color-mix(in srgb, var(--warn-line) 55%, transparent)" },
  ".cm-searchMatch-selected": { backgroundColor: "color-mix(in srgb, var(--warn-line) 40%, transparent)" },
  ".cm-panels": { backgroundColor: "var(--bg-subtle)", color: "var(--fg)", fontFamily: "var(--ui-font)", fontSize: "var(--ui-size)" },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-panels input, .cm-panels button": { fontFamily: "var(--ui-font)", fontSize: "var(--ui-size)" },
  // Find and replace panel (lib/findPanel.ts).
  ".fi-find": { display: "flex", flexDirection: "column", gap: "4px", padding: "6px 8px" },
  ".fi-row": { display: "flex", alignItems: "center", gap: "4px", minWidth: "0" },
  ".fi-input": {
    flex: "1",
    minWidth: "80px",
    maxWidth: "360px",
    height: "26px",
    padding: "0 8px",
    border: "1px solid var(--border-strong)",
    borderRadius: "5px",
    background: "var(--bg)",
    color: "var(--fg)",
    outline: "none",
    userSelect: "text",
  },
  ".fi-input:focus": { borderColor: "var(--fg-faint)", boxShadow: "0 0 0 1px var(--fg-faint)" },
  ".fi-input.fi-bad": { borderColor: "var(--danger)" },
  ".fi-toggle, .fi-icon": {
    display: "inline-grid",
    placeItems: "center",
    minWidth: "24px",
    height: "24px",
    padding: "0 4px",
    border: "1px solid transparent",
    borderRadius: "5px",
    background: "transparent",
    color: "var(--fg-muted)",
  },
  ".fi-toggle": { fontFamily: "var(--font-mono) !important", fontSize: "12px !important" },
  ".fi-word": { textDecoration: "underline", textUnderlineOffset: "2px" },
  ".fi-toggle:hover, .fi-icon:hover, .fi-btn:hover": { background: "var(--bg-hover)", color: "var(--fg)" },
  ".fi-toggle[aria-pressed=true]": { background: "var(--bg-active)", borderColor: "var(--fg-faint)", color: "var(--fg)" },
  ".fi-count": { minWidth: "64px", padding: "0 4px", color: "var(--fg-muted)", whiteSpace: "nowrap", fontVariantNumeric: "tabular-nums" },
  ".fi-count.fi-none": { color: "var(--danger)" },
  ".fi-btn": {
    height: "24px",
    padding: "0 8px",
    border: "1px solid var(--border-strong)",
    borderRadius: "5px",
    background: "var(--bg)",
    color: "var(--fg)",
    whiteSpace: "nowrap",
  },
  // Go to line (Mod+G), CodeMirror's dialog.
  ".cm-dialog": { display: "flex", alignItems: "center", gap: "6px", padding: "6px 8px" },
  ".cm-dialog label": { display: "flex", alignItems: "center", gap: "6px" },
  ".cm-dialog input": {
    width: "120px",
    height: "26px",
    padding: "0 8px",
    border: "1px solid var(--border-strong)",
    borderRadius: "5px",
    background: "var(--bg)",
    color: "var(--fg)",
    outline: "none",
    userSelect: "text",
  },
  ".cm-dialog button, .cm-button": {
    height: "24px",
    padding: "0 8px",
    border: "1px solid var(--border-strong)",
    borderRadius: "5px",
    background: "var(--bg)",
    backgroundImage: "none",
    color: "var(--fg)",
  },
  ".cm-dialog button[name=close]": { border: "0", background: "transparent", color: "var(--fg-muted)", fontSize: "16px" },
  ".cm-foldGutter span": { color: "var(--fg-faint)" },
  ".cm-changedLine": { backgroundColor: "var(--add-bg) !important" },
  ".cm-deletedChunk": { backgroundColor: "var(--del-bg) !important" },
  ".cm-insertedLine": { backgroundColor: "var(--add-bg) !important" },
  ".cm-deletedLine": { backgroundColor: "var(--del-bg) !important" },
  ".cm-changedText": { background: "color-mix(in srgb, var(--add) 25%, transparent) !important" },
  "del, .cm-deletedText": { background: "color-mix(in srgb, var(--del) 22%, transparent) !important", textDecoration: "none" },
  ".cm-collapsedLines": { backgroundColor: "var(--bg-subtle)", color: "var(--fg-muted)" },
});

export const highlight = syntaxHighlighting(
  HighlightStyle.define([
    { tag: [tags.keyword, tags.modifier, tags.operatorKeyword, tags.controlKeyword], color: "var(--syn-keyword)" },
    { tag: [tags.string, tags.special(tags.string), tags.regexp, tags.character], color: "var(--syn-string)" },
    { tag: [tags.number, tags.bool, tags.null, tags.atom], color: "var(--syn-number)" },
    { tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment], color: "var(--syn-comment)", fontStyle: "italic" },
    { tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.macroName], color: "var(--syn-func)" },
    { tag: [tags.typeName, tags.className, tags.namespace, tags.standard(tags.typeName)], color: "var(--syn-type)" },
    { tag: [tags.tagName, tags.heading], color: "var(--syn-tag)", fontWeight: "600" },
    { tag: [tags.attributeName, tags.propertyName], color: "var(--syn-prop)" },
    { tag: [tags.definition(tags.variableName), tags.constant(tags.variableName)], color: "var(--syn-def)" },
    { tag: [tags.meta, tags.processingInstruction], color: "var(--syn-comment)" },
    { tag: tags.link, textDecoration: "underline" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strong, fontWeight: "700" },
    { tag: tags.invalid, color: "var(--danger)" },
  ]),
);

/** The language name for a file name ("TypeScript"), or null when none is known. */
export function languageName(path: string): string | null {
  const name = path.split("/").pop() ?? path;
  if (name.endsWith(".svelte")) return "Svelte";
  return LanguageDescription.matchFilename(languages, name)?.name ?? null;
}

/** Language support for a file name, loaded lazily. */
export async function languageFor(path: string): Promise<Extension | null> {
  const name = path.split("/").pop() ?? path;
  let desc = LanguageDescription.matchFilename(languages, name);
  if (!desc && name.endsWith(".svelte")) desc = LanguageDescription.matchFilename(languages, name.replace(/\.svelte$/, ".html"));
  if (!desc) return null;
  try {
    return await desc.load();
  } catch {
    return null;
  }
}

/** CodeMirror's own words (go to line, folding, screen reader messages) from the string catalog. */
export const phrases = EditorState.phrases.of({
  "Go to line": t("editor.gotoLine"),
  go: t("editor.go"),
  close: t("common.close"),
  "Folded lines": t("editor.folded"),
  "Unfolded lines": t("editor.unfolded"),
  "folded code": t("editor.foldedCode"),
  unfold: t("editor.unfold"),
  "Fold line": t("editor.foldLine"),
  "Unfold line": t("editor.unfoldLine"),
  "current match": t("find.current"),
  "on line": t("find.onLine"),
  "replaced match on line $": t("find.replacedOn"),
  "replaced $ matches": t("find.replacedCount"),
  "Control character": t("editor.control"),
});

/** The smallest change that turns `from` into `to`, so the cursor and scroll stay put. */
export function minimalChange(from: string, to: string): { from: number; to: number; insert: string } | null {
  if (from === to) return null;
  let start = 0;
  const max = Math.min(from.length, to.length);
  while (start < max && from.charCodeAt(start) === to.charCodeAt(start)) start++;
  let endA = from.length;
  let endB = to.length;
  while (endA > start && endB > start && from.charCodeAt(endA - 1) === to.charCodeAt(endB - 1)) {
    endA--;
    endB--;
  }
  return { from: start, to: endA, insert: to.slice(start, endB) };
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
