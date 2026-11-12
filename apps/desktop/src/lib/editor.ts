// CodeMirror setup shared by the viewer and the diff viewer: theme from the app's color
// tokens (light and dark follow the CSS variables), language from the file name.
import { EditorView } from "@codemirror/view";
import { HighlightStyle, LanguageDescription, syntaxHighlighting } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { tags } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

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
  ".cm-searchMatch": { backgroundColor: "var(--warn-soft)", outline: "1px solid var(--warn-line)" },
  ".cm-panels": { backgroundColor: "var(--bg-subtle)", color: "var(--fg)" },
  ".cm-panels input, .cm-panels button": { fontFamily: "var(--ui-font)" },
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

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
