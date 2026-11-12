<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { basicSetup } from "codemirror";
  import { EditorView, keymap } from "@codemirror/view";
  import { Compartment, EditorState } from "@codemirror/state";
  import { unifiedMergeView } from "@codemirror/merge";
  import { highlight, languageFor, theme } from "../editor";

  let {
    path,
    text,
    original = null,
    readonly = false,
    onchange,
    onsave,
  }: {
    path: string;
    text: string;
    /** With an original, the editor shows a unified diff against it. */
    original?: string | null;
    readonly?: boolean;
    onchange?: (text: string) => void;
    onsave?: () => void;
  } = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  const lang = new Compartment();

  function build() {
    view?.destroy();
    if (!host) return;
    const extensions = [
      basicSetup,
      theme,
      highlight,
      lang.of([]),
      EditorView.lineWrapping,
      EditorState.readOnly.of(readonly),
      EditorView.editable.of(!readonly),
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: () => {
            onsave?.();
            return true;
          },
        },
      ]),
      EditorView.updateListener.of((u) => {
        if (u.docChanged) onchange?.(u.state.doc.toString());
      }),
    ];
    if (original !== null) {
      extensions.push(
        unifiedMergeView({
          original,
          mergeControls: false,
          highlightChanges: true,
          gutter: true,
          collapseUnchanged: { margin: 3, minSize: 6 },
        }),
      );
    }
    view = new EditorView({ state: EditorState.create({ doc: text, extensions }), parent: host });
    const forPath = path;
    languageFor(path).then((ext) => {
      if (ext && view && forPath === path) view.dispatch({ effects: lang.reconfigure(ext) });
    });
  }

  onMount(build);
  onDestroy(() => view?.destroy());

  // Rebuild when the file or the diff base changes.
  let last = { path: "", original: null as string | null, readonly: false };
  $effect(() => {
    const p = path;
    const o = original;
    const r = readonly;
    if (!view) return;
    if (p !== last.path || o !== last.original || r !== last.readonly) {
      last = { path: p, original: o, readonly: r };
      build();
    }
  });

  // Text replaced from outside (reload from disk).
  $effect(() => {
    const t = text;
    if (view && view.state.doc.toString() !== t && (original !== null || readonly || !view.hasFocus)) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: t } });
    }
  });

  onMount(() => {
    last = { path, original, readonly };
  });

  export function focus() {
    view?.focus();
  }
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
