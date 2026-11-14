// What FocusIDE reads from an agent's own TUI, in one place. These strings change when the
// agent updates, so the monthly check re-verifies them (AGENTS.md, "Monthly check";
// `cargo test -p focuside-desktop -- --ignored real_claude_mode_strings --nocapture`).
//
// Everything here fails quietly: no match means the feature is off (the mode button is
// disabled, the tab keeps its name). It never shows an error.

/**
 * Claude Code's status line for each permission mode, with the mode names hooks report.
 * Checked live on 2026-11-14 with Claude Code 2.1.323 on Windows 11 (a ConPTY and a vt100
 * screen), with U+23F8 or U+23F5 U+23F5 as the glyph: "<glyph> plan mode on (shift+tab to
 * cycle)", "<glyph> auto mode on (shift+tab to cycle)", "<glyph> manual mode on". Accept edits
 * ("accept edits on") and bypass permissions come from the permission modes docs and an earlier
 * live run. The mode glyph comes first.
 */
export const CLAUDE_MODE_LINES: readonly [RegExp, string][] = [
  [/[\u23f8\u23f5]\s*plan mode on/, "plan"],
  [/[\u23f8\u23f5]\s*accept edits on/, "acceptEdits"],
  [/[\u23f8\u23f5]\s*auto mode on/, "auto"],
  [/[\u23f8\u23f5]\s*manual mode on/, "default"],
  [/[\u23f8\u23f5]\s*bypass permissions on/, "bypassPermissions"],
  [/[\u23f8\u23f5]\s*don.t ask on/, "dontAsk"],
];

/** How many lines from the bottom of the screen hold Claude's status line. */
export const CLAUDE_STATUS_ROWS = 8;

/** The mode in `lines` (bottom line first), or null when no line matches. */
export function claudeMode(lines: string[]): string | null {
  for (const line of lines) for (const [re, mode] of CLAUDE_MODE_LINES) if (re.test(line)) return mode;
  return null;
}

/** The most Shift+Tab presses the plan mode switch sends for one click. Claude Code 2.1.323
 * cycles through 3 to 5 modes, depending on the user's settings. */
export const MODE_SWITCH_PRESSES = 4;

/** How long the switch waits for the status line to change after one press, in ms. */
export const MODE_SWITCH_WAIT = 1500;

/**
 * A tab name from a terminal title (OSC 0 or OSC 2), or null when the title is not a topic.
 * Claude Code sets the title to a topic it generates from the conversation (settings
 * reference, `terminalTitleFromRename`), with a status symbol in front while it works.
 * Rules:
 * - leading symbols (spinners, asterisks, dots, spaces) are removed;
 * - a path or a program file is not a topic (ConPTY sets the title to the program path at
 *   start, seen live: "C:\Users\...\claude.exe");
 * - the agent's own name is not a topic ("Claude Code", "claude", "codex", ...);
 * - the name is cut to TITLE_MAX characters.
 */
export function titleTopic(raw: string, agentNames: string[] = []): string | null {
  const t = raw.replace(/^[^\p{L}\p{N}]+/u, "").replace(/\s+/g, " ").trim();
  if (!t) return null;
  if (/^[A-Za-z]:[\\/]|^[\\/~]/.test(t) || /\.(exe|cmd|bat|ps1|js|mjs)$/i.test(t)) return null;
  const lower = t.toLowerCase();
  const names = ["claude", "claude code", ...agentNames.map((n) => n.toLowerCase())];
  if (names.includes(lower)) return null;
  return t.length > TITLE_MAX ? t.slice(0, TITLE_MAX - 3).trimEnd() + "..." : t;
}

export const TITLE_MAX = 60;

/**
 * The key a TUI reads as "paste the clipboard image", sent when the user pastes and the
 * clipboard has no text. Claude Code docs (interactive mode, keybindings `chat:imagePaste`):
 * Ctrl+V, or Alt+V on Windows. Alt+V reaches the program as ESC v. Other agents get Ctrl+V.
 */
export function imagePasteKey(agent: string | null, windows: boolean): string {
  return agent === "claude" && windows ? "\x1bv" : "\x16";
}
