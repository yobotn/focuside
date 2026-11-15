// Change markers in the editor gutter: lines added, changed or deleted against the HEAD
// version of the file (Git panel). The HEAD text comes in with the `setGitBase` effect; the
// markers then follow every edit (Chunk.updateB of @codemirror/merge, the same diff the diff
// view uses). A file that is not in HEAD, or a project without git, shows no markers.
import { RangeSetBuilder, StateEffect, StateField, Text, type Extension } from "@codemirror/state";
import { EditorView, GutterMarker, gutter } from "@codemirror/view";
import { Chunk } from "@codemirror/merge";

/** The HEAD text of the file, or null for no markers. */
export const setGitBase = StateEffect.define<string | null>();

/** Bounds the diff on big files, so typing stays fast. */
const DIFF = { scanLimit: 2000, timeout: 80 };

interface Base {
  text: Text | null;
  /** The raw HEAD text, so a reload with the same text changes nothing. */
  raw: string | null;
  chunks: readonly Chunk[];
}

const baseField = StateField.define<Base>({
  create: () => ({ text: null, raw: null, chunks: [] }),
  update(v, tr) {
    for (const e of tr.effects) {
      if (!e.is(setGitBase)) continue;
      if (e.value === v.raw) return v;
      if (e.value === null) return { text: null, raw: null, chunks: [] };
      const text = Text.of(e.value.split(/\r\n|\r|\n/));
      return { text, raw: e.value, chunks: Chunk.build(text, tr.state.doc, DIFF) };
    }
    if (v.text && tr.docChanged) return { ...v, chunks: Chunk.updateB(v.chunks, v.text, tr.state.doc, tr.changes, DIFF) };
    return v;
  },
});

class Mark extends GutterMarker {
  constructor(readonly kind: "add" | "mod" | "del") {
    super();
  }
  eq(other: GutterMarker) {
    return other instanceof Mark && other.kind === this.kind;
  }
  toDOM() {
    const el = document.createElement("div");
    el.className = `cm-git-${this.kind}`;
    return el;
  }
}
const ADD = new Mark("add");
const MOD = new Mark("mod");
const DEL = new Mark("del");

const markers = StateField.define({
  create: () => new RangeSetBuilder<GutterMarker>().finish(),
  update(v, tr) {
    if (!tr.docChanged && tr.startState.field(baseField) === tr.state.field(baseField)) return v;
    const { text, chunks } = tr.state.field(baseField);
    const b = new RangeSetBuilder<GutterMarker>();
    if (!text) return b.finish();
    const doc = tr.state.doc;
    for (const c of chunks) {
      if (c.fromB === c.toB) {
        // Lines removed: a mark on the line where they were.
        const at = Math.min(c.fromB, doc.length);
        b.add(doc.lineAt(at).from, doc.lineAt(at).from, DEL);
        continue;
      }
      const kind = c.fromA === c.toA ? ADD : MOD;
      const first = doc.lineAt(c.fromB).number;
      const last = doc.lineAt(Math.max(c.fromB, Math.min(c.endB, doc.length))).number;
      for (let n = first; n <= last; n++) {
        const line = doc.line(n);
        b.add(line.from, line.from, kind);
      }
    }
    return b.finish();
  },
});

const theme = EditorView.baseTheme({
  ".cm-git-gutter": { width: "3px", marginInlineStart: "2px" },
  ".cm-git-gutter .cm-gutterElement": { padding: "0" },
  ".cm-git-add, .cm-git-mod": { width: "3px", height: "100%" },
  ".cm-git-add": { background: "var(--add)" },
  ".cm-git-mod": { background: "var(--warn-line)" },
  ".cm-git-del": {
    width: "0",
    height: "0",
    borderTop: "4px solid transparent",
    borderBottom: "4px solid transparent",
    borderInlineStart: "5px solid var(--del)",
    marginTop: "-4px",
  },
});

/** The gutter with the change markers. Empty until `setGitBase` sets a text. */
export function gitGutter(): Extension {
  return [
    baseField,
    markers,
    gutter({
      class: "cm-git-gutter",
      markers: (v) => v.state.field(markers),
    }),
    theme,
  ];
}
