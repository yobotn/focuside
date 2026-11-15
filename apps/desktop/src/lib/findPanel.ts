// Find and replace in the open file: CodeMirror's search with the app's own panel. Mod+F finds,
// Mod+H replaces, Mod+G goes to a line. In the panel: Enter and Shift+Enter go to the next and
// previous match, Alt+C, Alt+W and Alt+R switch match case, whole word and regular expression,
// Mod+Alt+Enter replaces all, Esc closes and goes back to the text.
import {
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  gotoLine,
  openSearchPanel,
  replaceAll,
  replaceNext,
  search,
  SearchQuery,
  setSearchQuery,
} from "@codemirror/search";
import { Prec, type Extension } from "@codemirror/state";
import { EditorView, keymap, runScopeHandlers, type Panel, type ViewUpdate } from "@codemirror/view";
import { t } from "./i18n";
import { keyLabel } from "./platform";

const MAX_COUNT = 9999;

// Small line icons (Phosphor shapes, 16 px grid), static strings.
const ICONS = {
  up: '<svg viewBox="0 0 256 256" width="14" height="14" aria-hidden="true"><path fill="currentColor" d="M205.66 117.66a8 8 0 0 1-11.32 0L136 59.31V216a8 8 0 0 1-16 0V59.31l-58.34 58.35a8 8 0 0 1-11.32-11.32l72-72a8 8 0 0 1 11.32 0l72 72a8 8 0 0 1 0 11.32Z"/></svg>',
  down: '<svg viewBox="0 0 256 256" width="14" height="14" aria-hidden="true"><path fill="currentColor" d="m205.66 149.66-72 72a8 8 0 0 1-11.32 0l-72-72a8 8 0 0 1 11.32-11.32L120 196.69V40a8 8 0 0 1 16 0v156.69l58.34-58.35a8 8 0 0 1 11.32 11.32Z"/></svg>',
  close: '<svg viewBox="0 0 256 256" width="13" height="13" aria-hidden="true"><path fill="currentColor" d="M205.66 194.34a8 8 0 0 1-11.32 11.32L128 139.31l-66.34 66.35a8 8 0 0 1-11.32-11.32L116.69 128 50.34 61.66a8 8 0 0 1 11.32-11.32L128 116.69l66.34-66.35a8 8 0 0 1 11.32 11.32L139.31 128Z"/></svg>',
};

function el<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}, text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (text) e.textContent = text;
  return e;
}

class FindPanel implements Panel {
  dom: HTMLElement;
  top = true;
  private view: EditorView;
  private find: HTMLInputElement;
  private repl: HTMLInputElement;
  private toggles: Record<"caseSensitive" | "wholeWord" | "regexp", HTMLButtonElement>;
  private count: HTMLElement;
  private query: SearchQuery;
  private countTimer = 0;

  constructor(view: EditorView) {
    this.view = view;
    this.query = getSearchQuery(view.state);
    this.dom = el("div", { class: "fi-find", role: "search", "aria-label": t("find.title") });
    const row1 = el("div", { class: "fi-row" });
    this.find = el("input", { type: "text", class: "fi-input", "main-field": "true", placeholder: t("find.find"), "aria-label": t("find.find"), spellcheck: "false" });
    this.find.value = this.query.search;
    const toggle = (key: "caseSensitive" | "wholeWord" | "regexp", label: string, title: string, hint: string) => {
      const b = el("button", { type: "button", class: "fi-toggle", "aria-pressed": "false", title: `${title} (${keyLabel(hint)})`, "aria-label": title }, label);
      b.addEventListener("click", () => {
        this.set({ [key]: !this.query[key] });
        this.find.focus();
      });
      return b;
    };
    this.toggles = {
      caseSensitive: toggle("caseSensitive", "Aa", t("find.case"), "Alt+C"),
      wholeWord: toggle("wholeWord", "ab", t("find.word"), "Alt+W"),
      regexp: toggle("regexp", ".*", t("find.regex"), "Alt+R"),
    };
    this.toggles.wholeWord.classList.add("fi-word");
    this.count = el("span", { class: "fi-count", "aria-live": "polite" });
    const icon = (name: keyof typeof ICONS, label: string, hint: string, run: () => void) => {
      const b = el("button", { type: "button", class: "fi-icon", title: `${label} (${hint})`, "aria-label": label });
      b.innerHTML = ICONS[name];
      b.addEventListener("click", run);
      return b;
    };
    const prev = icon("up", t("find.prev"), "Shift+Enter", () => findPrevious(this.view));
    const next = icon("down", t("find.next"), "Enter", () => findNext(this.view));
    const close = icon("close", t("common.close"), "Esc", () => {
      closeSearchPanel(this.view);
      this.view.focus();
    });
    row1.append(this.find, this.toggles.caseSensitive, this.toggles.wholeWord, this.toggles.regexp, this.count, prev, next, close);
    this.dom.append(row1);

    this.repl = el("input", { type: "text", class: "fi-input", name: "replace", placeholder: t("find.replace"), "aria-label": t("find.replace"), spellcheck: "false" });
    this.repl.value = this.query.replace;
    if (!view.state.readOnly) {
      const row2 = el("div", { class: "fi-row" });
      const one = el("button", { type: "button", class: "fi-btn", title: `${t("find.replaceOne")} (Enter)` }, t("find.replaceOne"));
      one.addEventListener("click", () => replaceNext(this.view));
      const all = el("button", { type: "button", class: "fi-btn", title: `${t("find.replaceAll")} (${keyLabel("Mod+Alt+Enter")})` }, t("find.replaceAll"));
      all.addEventListener("click", () => replaceAll(this.view));
      row2.append(this.repl, one, all);
      this.dom.append(row2);
    }
    for (const input of [this.find, this.repl]) {
      input.addEventListener("input", () => this.commit());
      input.addEventListener("keydown", (e) => this.keydown(e, input === this.repl));
    }
    // Keys on the panel buttons: Esc closes, the search keys (F3, Mod+G) still work.
    this.dom.addEventListener("keydown", (e) => {
      if (e.defaultPrevented) return;
      if (e.key === "Escape") {
        e.preventDefault();
        closeSearchPanel(this.view);
        this.view.focus();
      } else if (runScopeHandlers(this.view, e, "search-panel")) e.preventDefault();
    });
    this.paint();
  }

  mount() {
    this.find.select();
    this.recount();
  }

  update(u: ViewUpdate) {
    for (const tr of u.transactions)
      for (const e of tr.effects) {
        if (e.is(setSearchQuery) && !e.value.eq(this.query)) {
          this.query = e.value;
          this.find.value = e.value.search;
          this.repl.value = e.value.replace;
          this.paint();
        }
      }
    if (u.docChanged || u.selectionSet || u.transactions.some((tr) => tr.effects.some((e) => e.is(setSearchQuery)))) this.recount();
  }

  /** Focus the replace field (Mod+H). */
  focusReplace() {
    if (this.view.state.readOnly) return;
    this.repl.focus();
    this.repl.select();
  }

  private set(change: Partial<{ caseSensitive: boolean; wholeWord: boolean; regexp: boolean }>) {
    const q = this.query;
    this.query = new SearchQuery({ search: this.find.value, replace: this.repl.value, caseSensitive: q.caseSensitive, wholeWord: q.wholeWord, regexp: q.regexp, ...change });
    this.view.dispatch({ effects: setSearchQuery.of(this.query) });
    this.paint();
  }

  private commit() {
    const q = new SearchQuery({ search: this.find.value, replace: this.repl.value, caseSensitive: this.query.caseSensitive, wholeWord: this.query.wholeWord, regexp: this.query.regexp });
    if (q.eq(this.query)) return;
    this.query = q;
    this.view.dispatch({ effects: setSearchQuery.of(q) });
    this.paint();
  }

  private paint() {
    for (const [k, b] of Object.entries(this.toggles)) b.setAttribute("aria-pressed", String(this.query[k as keyof typeof this.toggles]));
    this.find.classList.toggle("fi-bad", !!this.query.search && !this.query.valid);
  }

  /** "3 of 12", counted after typing stops for a moment. */
  private recount() {
    cancelAnimationFrame(this.countTimer);
    this.countTimer = requestAnimationFrame(() => {
      const q = this.query;
      if (!q.search) {
        this.count.textContent = "";
        return;
      }
      if (!q.valid) {
        this.count.textContent = t("find.invalid");
        return;
      }
      const state = this.view.state;
      const sel = state.selection.main;
      const cur = q.getCursor(state);
      let n = 0;
      let at = 0;
      for (let r = cur.next(); !r.done && n < MAX_COUNT; r = cur.next()) {
        n++;
        if (r.value.from === sel.from && r.value.to === sel.to) at = n;
      }
      this.count.textContent = n === 0 ? t("find.none") : at ? t("find.countAt", { index: at, count: n }) : t("find.count", { count: n });
      this.count.classList.toggle("fi-none", n === 0);
    });
  }

  private keydown(e: KeyboardEvent, replace: boolean) {
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === "Enter" && mod && e.altKey) {
      replaceAll(this.view);
    } else if (e.key === "Enter") {
      if (replace && !e.shiftKey) replaceNext(this.view);
      else (e.shiftKey ? findPrevious : findNext)(this.view);
    } else if (e.key === "Escape") {
      closeSearchPanel(this.view);
      this.view.focus();
    } else if (e.altKey && !mod && ["c", "w", "r"].includes(e.key.toLowerCase())) {
      const key = ({ c: "caseSensitive", w: "wholeWord", r: "regexp" } as const)[e.key.toLowerCase() as "c" | "w" | "r"];
      this.set({ [key]: !this.query[key] });
    } else if (mod && e.key.toLowerCase() === "h") {
      this.focusReplace();
    } else if (mod && e.key.toLowerCase() === "f") {
      this.find.select();
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }
}

const panels = new WeakMap<EditorView, FindPanel>();

/** Opens the panel and puts the cursor in the replace field. */
export function openReplace(view: EditorView): boolean {
  if (view.state.readOnly) return openSearchPanel(view);
  openSearchPanel(view);
  panels.get(view)?.focusReplace();
  return true;
}

/** The search setup with the app's panel and keys. Goes after basicSetup. */
export function findExtension(): Extension {
  return [
    search({
      top: true,
      createPanel: (view) => {
        const p = new FindPanel(view);
        panels.set(view, p);
        return p;
      },
    }),
    Prec.high(
      keymap.of([
        { key: "Mod-f", run: openSearchPanel, scope: "editor search-panel", preventDefault: true },
        { key: "Mod-h", run: openReplace, scope: "editor search-panel", preventDefault: true },
        { key: "Mod-g", run: gotoLine, preventDefault: true },
      ]),
    ),
  ];
}

export { gotoLine, openSearchPanel };
