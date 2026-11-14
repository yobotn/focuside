// Shared xterm.js setup for the terminal drawer and agent panes.
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { isDark } from "./theme.svelte";
import { isMac } from "./platform";

export function termColors() {
  const css = getComputedStyle(document.documentElement);
  const v = (n: string) => css.getPropertyValue(n).trim();
  return {
    background: v("--bg-sunken"),
    foreground: v("--fg"),
    cursor: v("--fg"),
    selectionBackground: isDark() ? "#3f3f46" : "#d4d4d8",
  };
}

/** PTY output arrives as base64 bytes. */
export function decode(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Clipboard text, or "" when the clipboard has no text (an image, or nothing). The Tauri
 * clipboard plugin first: WebView2 can ask the user before `navigator.clipboard.readText`. */
async function clipboardText(): Promise<string> {
  try {
    return (await readText()) ?? "";
  } catch {
    try {
      return (await navigator.clipboard.readText()) ?? "";
    } catch {
      return "";
    }
  }
}

export interface TermOptions {
  /** What a paste sends when the clipboard has no text, so the program can read the image
   * itself (see tui.ts, `imagePasteKey`). Default: Ctrl+V. */
  imageKey?: string;
}

/**
 * Paste, like Windows Terminal: the clipboard text goes in with `paste()` (bracketed paste when
 * the program asked for it). With no text on the clipboard the key goes to the program, so
 * Claude Code can paste an image on its own. Only on a user action (Ctrl+V or a right click).
 */
async function pasteInto(term: Terminal, imageKey: string) {
  const text = await clipboardText();
  if (text) term.paste(text);
  else term.input(imageKey, true);
}

function copySelection(term: Terminal) {
  const text = term.getSelection();
  term.clearSelection();
  if (text) writeText(text).catch(() => {});
}

export function newTerminal(scrollback = 5000, opts: TermOptions = {}): Terminal {
  const imageKey = opts.imageKey ?? "\x16";
  const term = new Terminal({
    fontFamily: '"Cascadia Code", "Cascadia Mono", Consolas, monospace',
    fontSize: 13,
    lineHeight: 1.2,
    cursorBlink: true,
    allowProposedApi: false,
    theme: termColors(),
    scrollback,
  });
  term.attachCustomKeyEventHandler((e) => {
    // Ctrl+Tab and Ctrl+Shift+Tab switch sessions, Ctrl+Shift+Page Up and Page Down move the
    // active session tab. xterm.js would send them to the program and stop the event, so it
    // skips them and they reach the app. Shift+Tab and Shift+Page Up alone still go to the
    // terminal.
    if (e.ctrlKey && e.key === "Tab") return false;
    if (e.ctrlKey && e.shiftKey && (e.key === "PageUp" || e.key === "PageDown")) return false;
    // macOS: Cmd+C and Cmd+V already copy and paste, and Ctrl+V is a key for the program.
    if (isMac || !e.ctrlKey || e.altKey || e.metaKey) return true;
    const k = e.key.toLowerCase();
    // Ctrl+V and Ctrl+Shift+V paste.
    if (k === "v") {
      if (e.type === "keydown") {
        e.preventDefault();
        pasteInto(term, imageKey);
      }
      return false;
    }
    // Ctrl+C copies when text is selected; otherwise it goes to the program (interrupt).
    // Ctrl+Shift+C always copies.
    if (k === "c" && (term.hasSelection() || e.shiftKey)) {
      if (e.type === "keydown") {
        e.preventDefault();
        copySelection(term);
      }
      return false;
    }
    return true;
  });
  return term;
}

/** Right click: copies the selection, or pastes when nothing is selected (as in Windows
 * Terminal). A two-finger tap on a touchpad is a right click. Returns a function that removes
 * the listener. */
export function rightClickPaste(term: Terminal, el: HTMLElement, opts: TermOptions = {}): () => void {
  const imageKey = opts.imageKey ?? "\x16";
  const onMenu = (e: MouseEvent) => {
    e.preventDefault();
    if (term.hasSelection()) copySelection(term);
    else pasteInto(term, imageKey);
    term.focus();
  };
  el.addEventListener("contextmenu", onMenu);
  return () => el.removeEventListener("contextmenu", onMenu);
}
