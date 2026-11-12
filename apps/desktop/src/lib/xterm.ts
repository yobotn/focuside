// Shared xterm.js setup for the terminal drawer and agent panes.
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { isDark } from "./theme.svelte";

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

export function newTerminal(scrollback = 5000): Terminal {
  return new Terminal({
    fontFamily: '"Cascadia Code", "Cascadia Mono", Consolas, monospace',
    fontSize: 13,
    lineHeight: 1.2,
    cursorBlink: true,
    allowProposedApi: false,
    theme: termColors(),
    scrollback,
  });
}
