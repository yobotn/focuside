// Every keyboard shortcut of the app in one table: the palette shows them on its rows and the
// Keyboard shortcuts sheet (Mod+/) lists them. Keys are written with "Mod" (Ctrl on Windows
// and Linux, Cmd on macOS); platform.keyLabel turns them into labels. The handlers live where
// the keys are used (App.svelte, the viewer, the cards); scripts/review.test.mjs checks the
// table for clashes. Type imports only, so Node can run it.
import type { Key } from "./i18n/en";

export type ShortcutGroup = "general" | "sessions" | "editor" | "review" | "cards";

export interface Shortcut {
  /** Palette command id, when the shortcut runs a palette command. */
  id: string;
  keys: string[];
  label: Key;
  group: ShortcutGroup;
  /** Where the key works, when not everywhere. */
  where?: Key;
}

export const SHORTCUTS: Shortcut[] = [
  { id: "palette", keys: ["Mod+K", "Mod+Shift+P"], label: "keys.palette", group: "general" },
  { id: "shortcuts", keys: ["Mod+/"], label: "keys.shortcuts", group: "general" },
  { id: "file", keys: ["Mod+P"], label: "cmd.openFile", group: "general" },
  { id: "search", keys: ["Mod+Shift+F"], label: "cmd.search", group: "general" },
  { id: "files", keys: ["Mod+B"], label: "cmd.files", group: "general" },
  { id: "term", keys: ["Mod+`"], label: "cmd.terminal", group: "general" },
  { id: "escape", keys: ["Esc"], label: "keys.escape", group: "general" },

  { id: "queue", keys: ["Mod+J"], label: "cmd.waiting", group: "sessions" },
  { id: "sessions", keys: ["Mod+E"], label: "cmd.sessions", group: "sessions" },
  { id: "nextSession", keys: ["Ctrl+Tab", "Ctrl+Shift+Tab"], label: "keys.nextSession", group: "sessions" },
  { id: "sessionN", keys: ["Mod+1-9"], label: "keys.sessionN", group: "sessions" },
  { id: "moveSession", keys: ["Ctrl+Shift+PageUp", "Ctrl+Shift+PageDown"], label: "keys.moveSession", group: "sessions" },
  { id: "stop", keys: ["Mod+."], label: "cmd.stop", group: "sessions" },
  { id: "runPlan", keys: ["Mod+Enter"], label: "keys.runPlan", group: "sessions", where: "keys.whereEmptyBox" },

  { id: "save", keys: ["Mod+S"], label: "keys.save", group: "editor" },
  { id: "saveAll", keys: ["Mod+Alt+S"], label: "cmd.saveAll", group: "editor" },
  { id: "closeEditor", keys: ["Mod+W"], label: "cmd.closeEditor", group: "editor" },
  { id: "reopen", keys: ["Mod+Shift+T"], label: "cmd.reopenEditor", group: "editor" },
  { id: "find", keys: ["Mod+F"], label: "cmd.findInFile", group: "editor" },
  { id: "replace", keys: ["Mod+H"], label: "cmd.replaceInFile", group: "editor" },
  { id: "gotoLine", keys: ["Mod+G"], label: "cmd.gotoLine", group: "editor" },
  { id: "editorTab", keys: ["Ctrl+PageUp", "Ctrl+PageDown"], label: "keys.editorTab", group: "editor", where: "keys.whereViewer" },

  { id: "nextChange", keys: ["J", "Alt+F5"], label: "keys.nextChange", group: "review", where: "keys.whereReview" },
  { id: "prevChange", keys: ["K", "Shift+Alt+F5"], label: "keys.prevChange", group: "review", where: "keys.whereReview" },
  { id: "undoChange", keys: ["Shift+U"], label: "review.undoChange", group: "review", where: "keys.whereReview" },

  { id: "review", keys: ["R"], label: "changes.review", group: "cards", where: "keys.whereCard" },
  { id: "undo", keys: ["U"], label: "changes.undo", group: "cards", where: "keys.whereCard" },
  { id: "discard", keys: ["D"], label: "keys.discard", group: "cards", where: "keys.whereCard" },
  { id: "focusCard", keys: ["Alt+ArrowUp"], label: "keys.focusCard", group: "cards", where: "keys.whereEmptyBox" },
];

export const GROUPS: { id: ShortcutGroup; label: Key }[] = [
  { id: "general", label: "keys.group.general" },
  { id: "sessions", label: "keys.group.sessions" },
  { id: "editor", label: "keys.group.editor" },
  { id: "review", label: "keys.group.review" },
  { id: "cards", label: "keys.group.cards" },
];

/** The keys of a palette command, or [] when it has none or they work only in one place. */
export function keysOf(id: string): string[] {
  const s = SHORTCUTS.find((x) => x.id === id);
  return s && !s.where ? s.keys : [];
}
