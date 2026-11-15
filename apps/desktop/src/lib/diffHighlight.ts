// Syntax colors for the review (ReviewView.svelte): the file's language parser from
// CodeMirror, with Lezer's class highlighter (tok-* classes, colored in ReviewView). One list
// of spans per line, so a diff line can show its colors without an editor.
import type { LanguageSupport } from "@codemirror/language";
import { classHighlighter, highlightTree } from "@lezer/highlight";
import { languageFor } from "./editor";

export interface Span {
  text: string;
  cls: string;
}

/** Larger texts show without colors. */
const MAX_CHARS = 400_000;

/** The spans of each line of `text` (index 0 is line 1), or null without a known language. */
export async function highlightLines(path: string, text: string | null): Promise<Span[][] | null> {
  if (!text || text.length > MAX_CHARS) return null;
  const ext = (await languageFor(path)) as LanguageSupport | null;
  const lang = ext?.language;
  if (!lang) return null;
  const tree = lang.parser.parse(text);
  const lines: Span[][] = [[]];
  let pos = 0;
  const take = (to: number, cls: string) => {
    while (pos < to) {
      const nl = text.indexOf("\n", pos);
      const end = nl < 0 || nl >= to ? to : nl;
      let piece = text.slice(pos, end);
      if (end === nl && piece.endsWith("\r")) piece = piece.slice(0, -1);
      if (piece) lines[lines.length - 1].push({ text: piece, cls });
      if (end === nl) {
        lines.push([]);
        pos = nl + 1;
      } else pos = end;
    }
  };
  highlightTree(tree, classHighlighter, (from, to, cls) => {
    take(from, "");
    take(to, cls);
  });
  take(text.length, "");
  return lines;
}
