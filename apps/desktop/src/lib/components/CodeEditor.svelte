<script lang="ts" module>
  import { Annotation, Compartment, EditorSelection, EditorState, Transaction, type Extension } from "@codemirror/state";
  import { EditorView, keymap, type ViewUpdate } from "@codemirror/view";

  /** Callbacks of the component that owns a view. Shared extensions look them up by view, so a
   * state cached for a tab works in a later editor too. */
  interface Hooks {
    update(u: ViewUpdate): void;
    save(): void;
  }
  const hooks = new WeakMap<EditorView, Hooks>();
  /** Marks text the editor takes from outside (load, reload): not an edit by the user. */
  const external = Annotation.define<boolean>();
  const lang = new Compartment();
  const listener = EditorView.updateListener.of((u) => hooks.get(u.view)?.update(u));
  const saveKeys = keymap.of([
    {
      key: "Mod-s",
      preventDefault: true,
      run: (v) => {
        hooks.get(v)?.save();
        return true;
      },
    },
  ]);
</script>

<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { basicSetup } from "codemirror";
  import { unifiedMergeView } from "@codemirror/merge";
  import { highlight, languageFor, minimalChange, phrases, theme } from "../editor";
  import { findExtension } from "../findPanel";
  import { editor, type Doc, type ViewPos } from "../stores/editor.svelte";

  let {
    path = "",
    text = "",
    original = null,
    readonly = false,
    doc = null,
    projectId = "",
    onchange,
    onsave,
    onedit,
  }: {
    path?: string;
    text?: string;
    /** With an original, the editor shows a unified diff against it. */
    original?: string | null;
    readonly?: boolean;
    /** An open file of an editor tab. The editor keeps its state (undo, cursor) per file. */
    doc?: Doc | null;
    projectId?: string;
    onchange?: (text: string) => void;
    onsave?: () => void;
    /** The user changed the text (pins a preview tab). */
    onedit?: () => void;
  } = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let ready = $state(false);
  /** The file the view shows (doc mode). */
  let shown: Doc | null = null;
  let lastRev = -1;
  let posTimer: ReturnType<typeof setTimeout> | null = null;

  function makeState(content: string, p: string, ro: boolean, orig: string | null, pos: ViewPos | null): EditorState {
    const ext: Extension[] = [
      basicSetup,
      findExtension(),
      theme,
      highlight,
      phrases,
      lang.of([]),
      EditorView.lineWrapping,
      EditorState.readOnly.of(ro),
      EditorView.editable.of(!ro),
      saveKeys,
      listener,
    ];
    if (orig !== null) {
      ext.push(
        unifiedMergeView({
          original: orig,
          mergeControls: false,
          highlightChanges: true,
          gutter: true,
          collapseUnchanged: { margin: 3, minSize: 6 },
        }),
      );
    }
    const max = content.length;
    const selection = pos ? EditorSelection.single(Math.min(pos.anchor, max), Math.min(pos.head, max)) : undefined;
    return EditorState.create({ doc: content, extensions: ext, selection });
  }

  function loadLanguage(p: string) {
    languageFor(p).then((ext) => {
      const cur = doc ? shown?.path : path;
      if (ext && view && cur === p) view.dispatch({ effects: lang.reconfigure(ext) });
    });
  }

  /** Text from outside: the smallest change, outside the undo history. */
  function applyText(next: string) {
    if (!view) return;
    const change = minimalChange(view.state.doc.toString(), next);
    if (change) view.dispatch({ changes: change, annotations: [external.of(true), Transaction.addToHistory.of(false)] });
  }

  function currentPos(): ViewPos | null {
    if (!view) return null;
    const s = view.state.selection.main;
    const top = view.lineBlockAtHeight(Math.max(0, view.scrollDOM.scrollTop)).from;
    return { anchor: s.anchor, head: s.head, top };
  }

  function reportPos() {
    if (posTimer) clearTimeout(posTimer);
    posTimer = setTimeout(() => {
      posTimer = null;
      const pos = currentPos();
      if (shown && pos) editor.setPos(projectId, shown.path, pos);
    }, 400);
  }

  /** Keeps the state of the file that leaves the view, for its tab. */
  function stash() {
    if (!view || !shown) return;
    shown.state = view.state;
    const pos = currentPos();
    if (pos) editor.setPos(projectId, shown.path, pos);
  }

  /** A pending jump to a line (Quick open, a search result) for the shown file. */
  function applyJump(): boolean {
    const j = editor.jump;
    if (!view || !shown || !j || j.projectId !== projectId || j.path !== shown.path) return false;
    editor.jump = null;
    const d = view.state.doc;
    const line = d.line(Math.max(1, Math.min(d.lines, j.line)));
    const from = Math.min(line.from + (j.col ?? 0), line.to);
    const to = Math.min(from + (j.len ?? 0), line.to);
    view.dispatch({ selection: { anchor: from, head: to }, effects: EditorView.scrollIntoView(from, { y: "center" }) });
    view.focus();
    return true;
  }

  function showDoc(d: Doc) {
    if (!view) return;
    if (shown && shown !== d) stash();
    shown = d;
    const fresh = !d.state;
    // Switching tabs from the keyboard (Ctrl+Page Down) keeps the focus in the text.
    const focused = view.hasFocus;
    view.setState(d.state ?? makeState(d.text, d.path, false, null, d.pos));
    if (focused) view.focus();
    d.state = null;
    lastRev = d.rev;
    if (view.state.doc.toString() !== d.text) applyText(d.text);
    if (fresh) loadLanguage(d.path);
    editor.view = view;
    if (!applyJump()) {
      const top = Math.min(d.pos?.top ?? 0, view.state.doc.length);
      view.dispatch({ effects: EditorView.scrollIntoView(top, { y: "start" }) });
    }
  }

  function buildPlain() {
    if (!view) return;
    view.setState(makeState(text, path, readonly, original, null));
    loadLanguage(path);
  }

  onMount(() => {
    if (!host) return;
    view = new EditorView({ parent: host });
    hooks.set(view, {
      update: (u) => {
        if (!u.docChanged && !u.selectionSet) return;
        if (doc && shown) {
          if (u.docChanged) {
            shown.text = u.state.doc.toString();
            if (u.transactions.some((tr) => tr.docChanged && !tr.annotation(external))) onedit?.();
          }
          reportPos();
        } else if (u.docChanged) onchange?.(u.state.doc.toString());
      },
      save: () => onsave?.(),
    });
    view.scrollDOM.addEventListener("scroll", () => doc && reportPos(), { passive: true });
    view.contentDOM.addEventListener("focus", () => {
      if (doc) editor.view = view;
    });
    if (doc) showDoc(doc);
    else buildPlain();
    last = { path, original, readonly };
    ready = true;
  });

  onDestroy(() => {
    if (posTimer) clearTimeout(posTimer);
    if (doc) stash();
    if (editor.view === view) editor.view = null;
    view?.destroy();
    view = null;
  });

  // Doc mode: another file, a reload, a jump.
  $effect(() => {
    const d = doc;
    if (!ready || !d) return;
    if (d !== shown) showDoc(d);
  });
  $effect(() => {
    const d = doc;
    const r = d?.rev;
    if (!ready || !d || d !== shown || r === lastRev) return;
    lastRev = r!;
    if (view && view.state.doc.toString() !== d.text) applyText(d.text);
  });
  $effect(() => {
    void editor.jump;
    if (ready && doc) applyJump();
  });

  // Plain mode: rebuild when the file or the diff base changes.
  let last = { path: "", original: null as string | null, readonly: false };
  $effect(() => {
    const p = path;
    const o = original;
    const r = readonly;
    if (!ready || doc) return;
    if (p !== last.path || o !== last.original || r !== last.readonly) {
      last = { path: p, original: o, readonly: r };
      buildPlain();
    }
  });
  // Plain mode: text replaced from outside.
  $effect(() => {
    const t = text;
    if (!ready || doc || !view) return;
    if (view.state.doc.toString() !== t && (original !== null || readonly || !view.hasFocus)) applyText(t);
  });

  export function focus() {
    view?.focus();
  }
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
