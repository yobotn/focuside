// Platform details for key labels and paths. Mod is Ctrl on Windows and Linux, Cmd on macOS.
export const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
export const isWindows = /Win/.test(navigator.platform || navigator.userAgent);

/** Label for a shortcut written with "Mod", for example "Mod+Enter" -> "Ctrl+Enter". */
export function keyLabel(combo: string): string {
  return combo.replace(/Mod/g, isMac ? "Cmd" : "Ctrl").replace(/Alt/g, isMac ? "Option" : "Alt");
}

/** True when the event has the platform's Mod key and no other modifier except the ones asked. */
export function mod(e: KeyboardEvent): boolean {
  return isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}
