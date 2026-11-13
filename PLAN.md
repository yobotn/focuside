# FocusIDE

The simplest IDE for working with AI agents. You read, approve and check. The agent types. Windows first, then macOS and Linux.

Website: [focuside.dev](https://focuside.dev). Repository: `github.com/yobotn/focuside` (public). License: GPL-3.0-only (changed from Apache 2.0 on 2026-11-12, before the first published release).

Status: M0 to M3 built and checked on Windows (see section 10). Next: M4, two weeks of daily use. Last updated 2026-11-12.

---

## TL;DR

- Rust + Tauri 2 + Svelte 5. CI builds Windows, macOS and Linux from M0. Windows ships first.
- One window. A rail of project monograms on the left, files, the conversation in the center, a viewer that opens only when needed.
- Plan card: at most 6 steps plus the files the agent will touch. Run it with one key. Type to change it.
- **Scope guard:** while the agent runs, files it touches that were not in the plan appear live in amber, with Pause.
- **Undo turn:** one key restores the project to how it was before the turn, git repo or not.
- **Waiting queue:** jump to the next session that needs you, across all projects.
- Agents connect through ACP (the protocol Zed uses). v1 is Claude Code only.
- Build order: the signature features first (about 2 weeks), then files and viewer, then polish. v1 is 4 to 5 weeks.

---

## 1. Product

**Problem.** Cursor has the right flow (plan, approve, apply) but is VS Code underneath with all of its noise. Zed and Warp are strong but heavy for this need. Claude Code in a terminal works, but long plans are hard to read, there is no file tree, and with several terminals open you lose track of which project and session each one is.

**Insight.** With agents, you mostly read and decide. You rarely type code. So the center of the screen is the conversation and its cards, not an editor.

**User.** First the owner. Then developers on any OS who run AI coding agents daily and want a quiet tool around them. Many are non-native English speakers.

**Promise.** Open a folder, ask, read a 6-line plan, run it, watch it stay in scope, undo it if you don't like it.

**Signature features:**
1. **Scope guard (plan vs actual, live).** During the run, every file outside the plan shows up in amber with the agent's reason and a Pause button. After the run, the changes card keeps that mark.
2. **Undo turn.** Restores every file the agent changed in the last turn, and tells the agent it was undone.
3. **Waiting queue.** Mod+J jumps to the next session that needs you, across all projects.

**The 20-second clip (marketing and the M3 acceptance test):** type "add dark mode". A 5-line plan appears. Mod+Enter. File rows tick one by one. `package.json` appears in amber: "Not in plan: added dependency". Undo turn. The tree snaps back: "Project is exactly as before."

**Out of scope for v1.** Language servers, debugger, extensions, collaboration, agents other than Claude Code, parallel agents in one project.

**Platforms.** Windows ships first. macOS and Linux are built and tested in CI from M0 and ship in v2.

### Landscape (checked 2026-11-11)

| Tool | Why it is not the answer |
|---|---|
| Cursor | VS Code based, noisy, paid |
| Zed | Close fit, but enterprise feel, many panels and settings |
| Warp | Terminal-first, over-engineered for this need |
| herdr (+ herdr-file-viewer) | Agent tabs and markdown in a terminal UI. No file icons, text only, Windows preview |
| Happy Desktop | All agents in one window. No file tree, viewer or plan card mentioned |
| Athas, Jean, cdesktop | Small projects, each covers part of the list |

Gap we fill: one calm app built around reading and approving agent work, with a live scope guard, undo turn, and always knowing where you are.

---

## 2. Rules

### Simplicity rules
- Core features (plan card, changes card, undo, viewer, editing) have no off switch. They are the product.
- Features appear in context: Git UI only inside a git repo, an agent only when it is installed.
- Settings in v1 has at most 6 rows: theme, font size, shell, default agent, show ignored files, notifications.
- If a choice has a sensible default, do not ask.
- No wizard, no welcome tour, no onboarding slides.

### No AI slop
- No purple gradients, sparkle icons, glowing orbs, "AI" badges, typing shimmer or chat bubbles.
- No emojis in UI, logs or commit messages. The root README.md may use them (marketing page).
- No em dashes. Use periods, commas or plain hyphens.
- No filler words ("seamless", "powerful", "unlock", "supercharge", "effortless", "robust", "magic").
- The agent's text is shown as a document, not as a chat.
- CI text lint fails on emojis or em dashes in UI strings (reuse Keeprun `scripts/text-lint.mjs`).

### Copy for non-native speakers
- Short, literal words. No idioms, no phrasal verbs.
- Words used: "Run plan", "Discard", "Pause", "Stop", "Undo turn", "Plan first", "Allow", "Allow in this chat", "Deny", "Open folder".
- Errors say what happened and what to do next. Example: "Claude Code is not installed. Copy the install command or run it in the terminal."
- Leave 30% extra width for translated text.
- Every string goes through an i18n file from day one (English only at first). RTL checked in v2.

### From Keeprun and RigSweep
- Nothing destructive without confirmation.
- Real measurements over claims: the changes card reads the disk, not the agent's words.

---

## 3. Visual identity

- **UI font:** the system font (Segoe UI Variable, SF Pro, Cantarell or system default), so it feels native everywhere.
- **Reading font:** IBM Plex Sans (OFL) at 15px, line height 1.6, for the conversation, cards and markdown. Not Inter or Geist (the default look of every AI app).
- **Code font:** Cascadia Code (OFL), bundled, so code looks the same on all three OSes.
- **UI icons:** Phosphor (MIT), Regular weight. More character than Lucide, the default of most AI apps.
- **File icons:** Symbols by Miguel Solorio (MIT): small, muted, consistent, one logo per file type. Build script copies only mapped SVGs. CI fails if the icon bundle grows over its size budget. A colorful set (Material Icon Theme) can come in v2.
- **Color:** Keeprun's zinc neutrals. Green = done and Run plan. Amber = waiting for you, and files not in the plan. Red = failed. The project color is the only other color on screen.
- **Project color:** hue generated from the folder path, so the same project always has the same color. Never green, amber or red.
- **Density:** 13px UI, 28px rows, hairline borders, no shadows except menus and dialogs.
- **Motion:** 120ms ease-out, only on state changes. Amber is the only thing that pulses.
- **Themes:** light, dark, follow system.
- **Recognizable screenshot:** the colored monogram rail plus the plan card's step and file grid with an amber row.
- **Quality bar:** Linear, Raycast, Things, GitHub Desktop.

---

## 4. Architecture

```
+----------------------------------------------------------------+
|  UI: Svelte 5 + Tailwind 4 (system webview)                     |
|  Rail | Files | Conversation + cards | Viewer/Diff | Terminal   |
+-------------------------------+--------------------------------+
                                | Tauri commands + events
+-------------------------------+--------------------------------+
|  Rust core                                                      |
|  fs: tree, watch, read, write, search    pty: terminal          |
|  agents: ACP client, one process per session                    |
|  proc: ProcessTree (Job Object on Windows, process group Unix)  |
|  snapshots: shadow git repo per project (changes + undo)        |
|  settings, per-project state                                    |
+-------------------------------+--------------------------------+
                                | stdio JSON-RPC (ACP)
                 claude-agent-acp (sidecar binary per OS)
                     -> the user's own `claude` install
```

### Agents: ACP only
- Protocol: ACP (Agent Client Protocol), the same setup Zed uses. Rust crate `agent-client-protocol` 2.x, pinned to an exact version.
- Claude Code: `@agentclientprotocol/claude-agent-acp`, shipped as a pinned standalone binary per OS (Tauri sidecar), so no Node is needed. `CLAUDE_CODE_EXECUTABLE` points it to the user's own `claude`.
- Capabilities are read from the `initialize` response. The UI hides what an adapter does not support (images, session list, resume, modes).
- No stream-json fallback. Known plan approval bugs over stdio make it a trap.
- Permission requests show the adapter's own options. FocusIDE keeps no permission rules of its own.
- "Run plan" answers the plan request with the adapter's "accept edits" option. Shell commands still ask.
- FocusIDE never reads, stores or proxies credentials. It only starts the user's installed agent.
- v2 adds Codex (`codex-acp` from `@agentclientprotocol/codex-acp`) and Gemini (`gemini --acp`, check the move to Antigravity CLI first).

### Plan card source (no second model call)
1. In plan mode, FocusIDE adds a short instruction to the session (how is decided in M0.5: adapter system prompt option if available): end every plan with `Steps:` (6 or fewer, plain words) and `Files:` (one line each: `create|edit|delete path - reason`).
2. The plan text comes from the `ExitPlanMode` request, which arrives as a permission request.
3. A deterministic parser reads `Steps:` and `Files:` and checks the paths on disk.
4. Fallback: the first 6 list items and the paths found in the text, with "The agent skipped the short plan. Showing the first 6 points."
5. ACP `plan` notifications (the agent's to-do list) are a different thing. They feed the live progress line.

### Scope guard
- During the run, ACP tool-call locations tell which files the agent reads and writes. Every write to a path that is not in the plan's file list appears live in the plan card in amber, with the agent's reason when it has one.
- Pause cancels the current prompt (ACP cancel). The plan card stays, with Continue and Undo turn.
- After the run, the changes card keeps the "Not in plan" mark, based on the real disk diff.

### Snapshots, changes and undo
- Each project has a shadow git repo in the app data folder: `--git-dir=<appdata>/projects/<hash>/shadow.git --work-tree=<project>`. It never touches the user's repo and works in folders that are not git repos.
- Snapshot when the project opens (in the background), before each turn, and at the end of each turn.
- Changes card: diff of turn start against turn end. Renames detected. Generated folders changed by commands (`node_modules`, `target`, `dist`) collapse to "+N generated files".

**Snapshot rules**
- Isolated git config: `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=<appdata>/gitconfig`, empty `core.hooksPath`, `core.autocrlf=false`, `core.filemode=false`, `core.longpaths=true`, `core.quotepath=false`, `core.symlinks=false` on Windows, `gc.auto=0`.
- Shadow `info/attributes`: `* -text -filter -ident`, so LFS and line-ending filters from the project never run.
- Nested `.git` folders and submodules are excluded and listed: "Undo does not cover <folder> (separate repo)".
- Files over 20 MB and symlinks on Windows are skipped and listed as "Not covered".
- Ignored files (`.env` and similar) are not snapshotted. If the watcher sees one change during a turn, the card shows "Changed, cannot undo".
- One queue per project for all shadow git calls, so no `index.lock` clashes.
- Refs `refs/fi/<session>/<turn>`. Keep the last 50 turns or 14 days, `gc` when the app is idle. Budget 1 GB per project, oldest turns pruned first.
- Locked or read-only files are reported per file. Nothing fails silently.

**Undo turn rules**
- 3-way restore: files changed again after the turn ended are listed as conflicts: "Changed after the turn: Keep mine / Revert anyway".
- Saves made in the FocusIDE viewer during a turn are labeled "your edit" and kept by default.
- A "before undo" snapshot is taken first, so Undo can itself be undone from the palette.
- The confirm dialog lists commands the turn ran (git commit, npm install, migrations) as "Not undone".
- The next prompt to the agent starts with: "The user reverted all file changes from your last turn."
- v1 offers Undo turn and revert one file. Revert one hunk comes in v2.

### One running turn per project
- v1 allows one running turn per project. Other sessions in the same project can plan. Their Run plan is disabled with "Another session is running here."
- If the watcher sees writes that the session's tool calls do not explain, the card says "Includes changes from outside this session."
- Parallel agents through git worktrees come in v2.

### Sessions
- ACP `session/list` and `session/resume` when the adapter supports them.
- Session names: generated from the first prompt, renameable, stored in FocusIDE's project state.
- Finding Claude Code sessions started in a normal terminal (reading its session files) comes in v2.

### Cross-platform
- **Process trees:** one `ProcessTree` layer for terminals and agents. Windows: Job Object with kill-on-close. Unix: new process group (`setsid`) and `killpg`, plus parent-death signal on Linux.
- **Finding agents:** on Windows resolve `.cmd` shims through PATHEXT. On macOS load the login shell's PATH (GUI apps do not get it).
- **Shells:** Windows `pwsh`, then `powershell`. Unix `$SHELL -l`.
- **Data folders:** Tauri `app_data_dir` and `app_config_dir`, never hardcoded paths. Respect `CLAUDE_CONFIG_DIR`.
- **Webviews:** WebView2, WKWebView, WebKitGTK. On Linux use xterm's DOM or canvas renderer (not WebGL) and document the NVIDIA `WEBKIT_DISABLE_DMABUF_RENDERER=1` workaround. Test Shiki on WebKit engines.
- **Title bar:** Windows uses the native caption so Snap Layouts work. macOS uses the overlay title bar with content after the traffic lights, and a native menu bar including Edit (needed for copy and paste).
- **Packaging (v2/v3):** Windows signing with Azure Artifact Signing. macOS Developer ID, notarization, universal build. Linux AppImage plus .deb. `tauri-plugin-updater` with `latest.json` on focuside.dev.
- **CI:** windows-latest, macos-latest, ubuntu-22.04 from M0.

### Security
- Agent output is untrusted. Markdown is sanitized (DOMPurify) before it is shown.
- Strict Content Security Policy in `tauri.conf`. Only the Tauri commands the UI needs are exposed.

### Packages
- Rust: `tauri` 2, `agent-client-protocol` (pinned), `portable-pty`, `ignore`, `grep-searcher`, `notify` with debouncer, `similar`, `serde`, `tokio`, `anyhow`, `tracing`. Shadow repo through the `git` command line (simple, proven).
- UI: `svelte` 5, `tailwindcss` 4, Phosphor icons, `codemirror` 6 with `@codemirror/language-data`, `markdown-it` + DOMPurify, `shiki` fine-grained bundle with lazy grammars, `@xterm/xterm`, a virtualized list for long conversations.

---

## 5. UX

### Default layout (one window)

```
+--+---------------------------------------------------------------------+
|KR| keeprun  main  .  Add version flag  .  Claude Opus    2 waiting Mod+J|
+--+------------+--------------------------------+-----------------------+
|KR| FILES      |  > add a --version flag         | diff  src/cli.rs  1/3 |
|RS| CLAUDE.md  |                                 | @@ -12 +12,4 @@       |
|  | > apps     |  [ plan card ]                  | + .version(..)        |
|+ | v crates   |                                 |                       |
|  |   cli.rs   |  [ changes card ]               |                       |
|  |   main.rs  |                                 |                       |
|  |            |  Message   @ files   / commands |                       |
+--+------------+--------------------------------+-----------------------+
```

- **Rail** (far left): one colored monogram per open project (KR = keeprun, RS = rigsweep). Click to switch project. Amber dot when a session there waits for you. "+" opens a folder.
- **Identity bar** (top): project, branch, session name, agent and model (click to change model), context usage when the adapter reports it, waiting count.
- **Window title:** `project - session name - FocusIDE`, plus the total waiting count, so the taskbar, Dock and Alt+Tab show it.
- **Files** (left): open by default, Mod+B hides it. CLAUDE.md pinned at the top when it exists.
- **Conversation** (center): max 760px wide, reads like a document. Tool calls show as one line ("Read src/main.rs"). Long conversations use a virtualized list.
- **Viewer** (right): opens only when a file or diff is opened. Code in CodeMirror, markdown rendered, toggle to source. If the agent changes a file with unsaved edits: "Changed on disk. Reload / Keep mine".
- **Terminal:** bottom drawer, one terminal in v1, hidden until opened (Mod+`).

### Message box
- Enter sends. Shift+Enter adds a new line.
- Mod+Enter with an empty box runs the waiting plan. With text, it sends the text as a change to the plan.
- `@` opens a file picker (sent as a file link to the agent). `/` lists the agent's commands, including custom commands and skills.
- Paste or drop images when the agent supports them.
- Up recalls earlier prompts. Drafts are kept per session.
- **Plan first** toggle in the box, remembers the last state.

### Plan card

```
+ Plan . 3 steps . 3 files ------------------------- waiting for you +
| 1  Add --version flag to the CLI parser                           |
| 2  Read the version from Cargo.toml at build time                 |
| 3  Add 1 test                                                     |
|                                                                   |
| edit    src/cli.rs          parse the flag                        |
| edit    src/main.rs         print and exit                        |
| create  tests/version.rs    new test                              |
|                                                                   |
| Risk: none found                                                  |
| [Run plan  Mod+Enter]   [Discard  D]               Full plan  F   |
+-------------------------------------------------------------------+
```

- **States:** Planning (one live line, no shimmer), Waiting (amber), Running (file rows tick when touched, files not in the plan appear in amber with Pause), Paused (Continue, Undo turn), Done (the card becomes the changes card in place), Failed (error, Retry, Undo turn), Discarded (one line).
- More than 6 steps: the card shows 6 and "+N more".
- Single-letter keys work only when the card has focus. Alt+Up moves focus from the message box to the last card.

### Changes card

```
+ Changes . 4 files . +38 -6 ---------------------------------- done +
| edit    src/cli.rs          +12 -2                                |
| edit    src/main.rs         +4  -1                                |
| create  tests/version.rs    +22                                   |
| edit    package.json        +1      Not in plan: added dependency |
| +214 generated files (target/)                                    |
| [Review  R]   [Undo turn  U]                                      |
+-------------------------------------------------------------------+
```

- Review opens the diff viewer. J/K next or previous file. Revert one file from the viewer.

### Always know where you are
- One window, rail of projects, identity bar, window title (above).
- Project color and monogram on the rail and the identity bar.
- Session names from the first prompt, renameable with a double click.
- Warning if an agent runs in a different folder than the project.
- **Waiting queue (Mod+J):** all sessions waiting for you, across all projects. Enter jumps there.
- **Sessions list (Mod+E):** sessions for this project. Click to resume.
- System notification only when a session in a project you are not looking at waits for you.

### First run
- Check for `claude` on PATH, whether it is signed in, and `git` (needed for Undo turn).
- Window shows recent projects, one "Open folder" button, and drag a folder onto the window. `focuside .` opens a folder from any terminal.
- After a folder opens, focus goes straight to the message box with Plan first on.
- Something missing: one row with the install command, Copy and "Run in terminal".
- Not signed in: "Sign in to Claude Code" opens the terminal drawer with `claude` running.
- Agent process died: "Agent stopped. Resume".

### Keys
Mod is Ctrl on Windows and Linux, Cmd on macOS.

| Action | Key |
|---|---|
| Command palette | Mod+K |
| Open file | Mod+P |
| Search in files | Mod+Shift+F |
| Run plan | Mod+Enter (empty message box) |
| Discard plan | D (card focused) |
| Stop agent | Mod+. (and Esc when nothing is open). Kills the process tree if it has not stopped after 3 s |
| Undo turn | U (changes card focused), or palette |
| Waiting queue | Mod+J |
| Sessions | Mod+E |
| Files | Mod+B |
| Terminal | Mod+` |

- Esc only closes overlays and the viewer. It never discards a plan.
- When the terminal has focus, only Mod+` and Ctrl+Shift+key reach the app. Everything else goes to the shell.
- Mod+Z is always normal undo in text.

---

## 6. Milestones

Build the signature slice first. It is what makes FocusIDE worth using.

| Phase | Content | Estimate |
|---|---|---|
| M0 Setup | Repo, Tauri app opens, 3-OS CI, lint, empty state | 1 day |
| M0.5 Agent spike | Rust CLI, no UI: Claude Code over ACP, plan mode, permissions, cancel, resume, clean kill, how to add the plan instruction | 1 to 2 days |
| M1 Signature slice | Conversation, plan card, scope guard, shadow snapshots, changes card, undo turn, rail with waiting dots, Stop | 8 to 10 days |
| M2 Files and viewer | File tree with icons, viewer, markdown, diff viewer, search in files, @ files | 4 to 5 days |
| M3 Daily essentials and polish | / commands, images, model picker, sessions, terminal drawer, first run, settings, themes, palette | 4 to 5 days |
| M4 Daily use | Owner uses it for 2 weeks. Annoyances go to `docs/daily-use.md` | 2 weeks |
| v2 | macOS and Linux releases, Codex and Gemini, revert hunk, terminal tabs, outside sessions, parallel agents with worktrees, Material icons, MCP, Mermaid | later |
| v3 | Keeprun and RigSweep extras, signing, auto-update, website on focuside.dev | later |

### M0 tasks
1. Check tools: `rustc`, `cargo`, `node`, `npm`, `git`, WebView2. If something is missing, stop and report the install command. Do not install without approval.
2. `git init`, `.gitignore`, `README.md`, `LICENSE` (Apache 2.0, copy Keeprun's).
3. Scaffold Tauri 2 + Svelte 5 + Tailwind 4, configs modeled on Keeprun's `apps/desktop`.
4. Text lint from Keeprun. `npm run lint`: clippy, fmt, text lint, svelte-check.
5. GitHub Actions workflow with the 3-OS build matrix (runs once the repo exists).
6. Window opens with the empty state, native title bar rules from section 4.
7. Commit.

**Done when:** `npm run dev` opens the app on Windows and `npm run lint` passes.

### M0.5 tasks
1. `ProcessTree` layer (Windows Job Object now, Unix version compiled in CI).
2. Find `claude` (PATHEXT on Windows). Download the pinned `claude-agent-acp` binary for Windows.
3. Small Rust CLI: start the adapter, `initialize`, print capabilities, new session, stream a reply.
4. Trigger a permission request and answer it.
5. Plan mode: capture the `ExitPlanMode` plan text, answer Run or Discard, confirm the turn continues after Run.
6. Find how to add the plan format instruction (adapter system prompt option or another way that does not write into the user's repo).
7. Cancel a running prompt. Resume a session. Kill the process tree and confirm nothing is left.
8. Record tool-call locations for reads and writes (needed for the scope guard).
9. Write all results in section 10.

**Done when:** every step works, or the blocker is written down with a workaround.

### M1 tasks
1. Agent manager in Rust on top of the spike (start, stream, cancel, close, resume).
2. Conversation view: document style, one-line tool calls, Stop.
3. Permission row with the adapter's options.
4. Plan first toggle, plan card states and keys, parser and fallback.
5. Scope guard: live amber rows, Pause, Continue.
6. Shadow repo with all snapshot rules, per-project queue, pruning.
7. Changes card, Undo turn with all undo rules, revert one file.
8. One running turn per project.
9. Rail with project colors and waiting dots, identity bar, window title, waiting queue.

**Done when:** the 20-second clip from section 1 works end to end in a test repo, and after Undo turn the folder is byte for byte as before.

### M2 tasks
1. Rust: lazy `list_dir` with `ignore`, `notify` watcher with debounce, search with `grep-searcher`.
2. Icon build script for Symbols, size budget check.
3. File tree: expand, collapse, keyboard navigation, icons, CLAUDE.md pinned.
4. Viewer: CodeMirror with language from the file name, Mod+S saves. Markdown with Shiki and DOMPurify, toggle to source. Large or binary files show size and "Open with system app". "Changed on disk" banner.
5. Diff viewer for the changes card.
6. `@` file picker in the message box.

**Done when:** the Keeprun repo shows correct icons for `.rs`, `.ts`, `.svelte`, `.md`, `.toml`, `.json`, `Cargo.toml`, `package.json`, PLAN.md renders well, and Mod+Shift+F finds text across the repo.

### M3 tasks
1. `/` commands from the agent, image paste and drop, model picker, context usage.
2. Sessions list and resume, session rename, prompt history, drafts.
3. Terminal drawer: one terminal, `portable-pty`, xterm.js, clean kill on close.
4. First run checks and messages, "Agent stopped. Resume".
5. Settings (6 rows), themes, command palette, app icon, notifications.

---

## 7. Working agreement for unattended sessions

- Work milestone by milestone, in order. Commit after each task with a short literal message.
- Run `npm run lint` and `cargo test` before each commit. Never skip hooks.
- Do not install system tools (Rust, Node, WebView2, git) without approval. Project npm and cargo packages are fine.
- Create the public repo `yobotn/focuside` only after the owner says go. Until then, commit locally only.
- When blocked, write the blocker in section 10 and move to the next task that does not depend on it.
- End of session: update the Status line at the top and section 10.

---

## 8. Risks

| Risk | Mitigation |
|---|---|
| Claude subscription use through the Agent SDK (which the ACP adapter uses) counts against a separate Agent SDK credit, and third-party apps may not offer Claude.ai login without approval | FocusIDE never handles credentials, it only starts the user's own `claude`. Check Anthropic's policy before public release. A Claude pane (the unmodified `claude` TUI with its own login) stays available |
| ACP adapter changes or lacks a feature | Pinned versions, capability checks, M0.5 spike before any UI |
| No way to add the plan format instruction | Fallback parser (first 6 points and paths). Decided in M0.5 |
| Undo misses something (ignored files, nested repos, commands) | Every uncovered item is listed in the card and the confirm dialog. Never claim more than was restored |
| Shadow snapshots slow or large on huge repos | Git index is incremental, `.gitignore` respected, 20 MB file limit, 1 GB budget, idle `gc` |
| Feature creep turns it into Zed | Core has no switches, Settings max 6 rows, v2 only from the daily-use list |
| WebKitGTK and WKWebView differences | 3-OS CI from M0, bundled fonts, xterm renderer per OS |
| Untrusted agent output in a webview | DOMPurify, strict CSP, minimal Tauri commands |
| Estimates too low | Signature slice first, so the valuable part exists even if later phases slip |

---

## 9. Brand

- Name: **FocusIDE**. Written as one word in brand casing (FocusIDE, FocusIDE.dev). Spoken "Focus IDE".
- Main domain: `focuside.dev` (website, downloads, later `latest.json` for updates). .dev is HTTPS only, which GitHub Pages supports.
- Protection domain: `focuside.app`, redirects to focuside.dev, through registrar forwarding with HTTPS or a tiny GitHub Pages redirect repo. No Cloudflare or other edge services (same rule as Keeprun).
- Repository: `github.com/yobotn/focuside`, public.
- Desktop binary: `FocusIDE` (`FocusIDE.exe` on Windows). CLI shim: `focuside`.
- Website: GitHub Pages from the repo, deployed by Actions, same setup as keeprun.app. The hero is the 20-second clip.
- Family: Keeprun (keep apps running), RigSweep (room for your agents), FocusIDE (work with your agents).
- To verify: trademark search (USPTO / EUIPO) for "FocusIDE" and "Focus IDE".

### Names considered (2026-11-11)
| Name | Why not |
|---|---|
| Quiet IDE | Non-native speakers confuse "quiet" and "quite" |
| Simple IDE | SimpleIDE already exists (Parallax), too generic |
| Light IDE | Sounds like LiteIDE (Go IDE) |
| Haven IDE | Runner-up, fine but less direct |
| Plainpane, Rigpane, Calmdock and others | Two English words glued together confuse non-native speakers |
| Calmo, Codalo, Simplo, Kalido | Made-up names, owner preferred a real word |

---

## 10. Decisions and spike results

| Decision | Why |
|---|---|
| Build our own instead of herdr, Happy, Zed or Warp | Need minimal UX plus file icons, readable markdown and a plan card together |
| Stack matches Keeprun (Tauri, Svelte, Tailwind) | Shared knowledge, configs and lint scripts across the product family |
| Cross-platform, Windows first, 3-OS CI from M0 | Owner requirement. Unix must not break silently |
| One window with a project rail | Simpler than one window per project, still always shows where you are |
| Conversation in the center, files left, viewer on demand | With agents you read and approve more than you type |
| ACP only, Claude Code only in v1 | Same path as Zed. stream-json has known plan approval bugs |
| Own ACP client on serde_json instead of the SDK crate | Small client surface, no churn from SDK major versions (decided in M0.5) |
| Agent spike before UI, signature slice before files | Riskiest unknown first, most valuable part first |
| Short plan from a fixed format and a parser, no second model call | No extra cost, delay or billing, no extra session files |
| Scope guard live during the run | Turns plan vs actual from a report into a guard. The shareable moment |
| Shadow git repo in app data, with strict rules | Exact per-turn diffs and undo, works without git, never touches the user's repo |
| One running turn per project in v1 | Keeps diffs, scope guard and undo correct |
| Esc never discards, Mod+Z stays text undo, terminal keeps its keys | No accidental loss, no fights with muscle memory |
| Core features have no switches, Settings max 6 rows | Avoids a Zed-style settings page |
| Phosphor UI icons, Symbols file icons | Calm and distinctive, avoids the default AI-app look |
| Plain words: Run plan, Discard, Pause, Undo turn, Plan first | Clear for non-native speakers. "Build" reads as compile |
| Name FocusIDE, focuside.dev main, focuside.app redirect | Real word, easy in any language, says the promise |
| GPL-3.0-only, public repo | Changed from Apache 2.0 by the owner on 2026-11-12. Forks must stay open source; the owner, as the only copyright holder, can still offer commercial licenses |
| Always know where you are is a core requirement | Owner's long-standing pain with Claude Code in terminals |

### Spike results (M0.5, 2026-11-12, claude-agent-acp 0.106.0, Claude Code 2.1.322, Windows 11)

| Step | Result |
|---|---|
| ACP client | Written directly on `serde_json` (JSON-RPC, one message per line), not the `agent-client-protocol` crate. The crate is at 5.x now and its API moves between majors. The client side is about 300 lines in `crates/core/src/acp.rs` |
| Adapter | Release binary `claude-agent-acp-windows-x64.zip` (113 MB exe). `scripts/fetch-adapter.mjs` puts it in `src-tauri/binaries` as a Tauri sidecar. `CLAUDE_CODE_EXECUTABLE` points it to the user's `claude.exe` (`~/.local/bin`). npm `.cmd` shims are followed to the real program |
| initialize | Under 2 s. Capabilities: loadSession, image, sessionCapabilities list, resume, close, fork, systemPrompt (append and override), title, status. `authMethods` empty when signed in |
| Sign-in | `auth/status` request and `auth/status_update` notifications give `state: authenticated` |
| Plan instruction | Solved: `session/new` takes `systemPrompt: {mode: "append", content: [text]}`. Nothing is written to the user's repo. Claude followed the `Steps:` / `Files:` / `Risk:` format on every test and the parser read it |
| Plan request | Arrives as `session/request_permission` with `toolCall.kind = "switch_mode"`, title "Approve Plan", plan text in `toolCall.rawInput.plan`. Options: `exit-plan-default`, `exit-plan-clear-auto`, `exit-plan-auto`, `exit-plan-bypass`, `reject`. There is no accept-edits option when `auto` mode exists |
| Run plan | Answer `exit-plan-default`, wait for `current_mode_update` (default), then `session/set_mode acceptEdits`. Setting the mode before that update loses a race and edits still ask. With the wait, edits went through and shell commands kept their normal rule |
| Discard | Answer `reject`, then `session/cancel`. Prompt ends with `stopReason: cancelled` |
| Cancel | Works, but the agent waits for a running shell command: cancel after 12 s of `ping -n 40` ended at 34 s. Stop and Pause kill the tree after 3 s, as planned |
| Resume | `session/resume` with the old session id keeps the full history |
| Kill | The Job Object held 10 processes (adapter, claude, shells). All gone after `TerminateJobObject` |
| Tool locations | `tool_call` has `kind` (read, edit, delete, move, execute, search, switch_mode, other). Paths come later in `tool_call_update.locations` as absolute paths, without `kind`, so kind is tracked by `toolCallId`. Titles are short ("Read src\greet.js", "Edit package.json", the shell command) |
| Writes outside the project | Plan mode writes the plan to `~/.claude/plans/*.md`. The scope guard ignores paths outside the project root |
| Other updates | `session_info_update.title` (a session name from the agent), `usage_update` (`used`/`size` for context, `cost`), `config_option_update` with `mode`, `model`, `effort`. Model is changed with `session/set_config_option` (`configId: "model"`). `available_commands_update` lists slash commands and skills |
| Default mode | Follows the user's Claude Code setting (here `auto`). FocusIDE switches to `plan` for Plan first and to `acceptEdits` after Run plan, and restores the session's original mode for prompts without Plan first |

---

### Build results (M1 to M3, 2026-11-12)

| Check | Result |
|---|---|
| M1 done-when: the 20-second clip | Passed in a test repo. "add dark mode" gave a 5-step, 3-file plan. Mod+Enter ran it, file rows ticked, the changes card replaced the plan card in place. A second run that wrote CHANGELOG.md outside the plan showed it live in amber with Pause, and the changes card kept "Not in plan". Undo turn: every file matched the committed blob (SHA-1), the created file was removed, "Project is exactly as before" |
| Run plan edits | Went through without asking. Shell commands still asked (permission row with the adapter's options) |
| Stop | Mod+. ended the turn in about 1 s and killed the running ping. The 3 s tree kill is in place for agents that do not stop |
| Agent killed from outside | "Agent stopped. Resume" appeared. Resume started the agent and resumed the session |
| Resume from the sessions list | `session/load` replays the history into the conversation |
| M2 done-when on Keeprun | Icons correct for .rs, .ts, .svelte, .md, .toml, .json, Cargo.toml (Rust icon), package.json. PLAN.md renders well. Mod+Shift+F found "fn main" across the repo and skipped ignored folders |
| M3 | Terminal drawer (pwsh in the project folder), palette, light and dark themes, model picker from the adapter's config options, @ file picker, settings (6 rows), session list |
| Tests | 24 Rust tests: plan parser, process tree kill, ACP round trip, run-plan option choice, snapshots (byte-for-byte undo with CRLF, binary, rename and generated folders; redo; conflicts; nested repos and big files; ignored files; pruning), file listing and search |

Fixes found while checking:
- Watching a big tree with notify-debouncer-full scanned the whole folder first (Keeprun's target/ took over 10 s). Switched to notify-debouncer-mini and moved watcher setup off the open path.
- `git check-ignore` rejects `--literal-pathspecs`, so ignored-file detection returned nothing. The flag is now skipped for that command.
- Sync Tauri commands run on the main thread, outside the Tokio runtime, so Stop crashed the app. Spawns now use `tauri::async_runtime`.
- In dev, a hot reload ran the app setup again and every agent message showed three times. Setup now runs once.

Not checked live yet (code is in place):
- Two sessions in one project running at the same time (Run plan disabled with "Another session is running here.").
- System notifications for a session in another project.
- Image paste and drop (the adapter reports image support).
- Sign-in flow when Claude Code is signed out.
- macOS and Linux: built in CI only. The Unix process group code has not run on a real machine.

Not done in v1, as planned or for packaging:
- The `focuside` command line shim. The app already opens folders passed as arguments (`FocusIDE.exe <folder>`) and forwards them to the running window. Adding the shim to PATH belongs to packaging (v2/v3).
- Revert one hunk (v2).

### After M3: owner feedback (2026-11-12)

The owner asked for three things after the first build: more agents than Claude, readable markdown files, and panels that resize and move.

- **More agents (pulled forward from v2).** Settings > Default agent lists Claude Code, Codex (`codex-acp`), Gemini CLI (`gemini --acp`, was `--experimental-acp` until 2026-11-12), OpenCode (`opencode acp`), Qwen Code (`qwen --acp --experimental-skills`, was `--experimental-acp`), Goose (`goose acp`) and Custom command. Each one shows "(not installed)" when it is not on PATH. The palette has "New session with <agent>" for each installed agent. A custom command can start with `NAME=value` parts, which set environment variables. Sessions remember their agent, so resume uses the same one. "Plan first" is turned off for agents that do not report a plan mode. Snapshots, undo and the changes card do not depend on the agent.
  - Checked live: the picker, a custom command (the spawn path ran and the agent's own error came back), the agent name in the bar and in errors.
  - Not checked live: Codex, Gemini, OpenCode, Qwen and Goose themselves (none are installed on this machine). ACP flags may change upstream; the 2026-11-12 values come from the ACP registry.
- **Markdown files.** Files render as documents: embedded HTML (sanitized), YAML front matter as a small table, task lists, GitHub alerts (`> [!NOTE]`), `<details>`, `<kbd>`, `align="center"`, heading anchors, links between .md files (open in the viewer) and images inside the project (loaded through `fs_image`, 8 MB limit). Web images are not fetched (privacy and the content policy); they show as a chip with their alt text that opens the link. Agent text still renders without raw HTML.
- **Panels.** The file tree, viewer and terminal resize by dragging the 1 px divider (arrow keys also work; double-click resets). The file tree and viewer each move to the other side with the arrows button in their header or from the palette ("Move file tree/viewer to the other side", "Reset panel layout"). Sizes and sides are saved. The conversation keeps at least 360 px.
- **State file safety.** A `state.json` that does not parse (a hand edit with a BOM wiped the test machine's recent list) is now kept as `state.json.bad`. The app falls back to `state.json.bak`, which every save keeps one version of.

### Anthropic policy check (2026-11-12)

Sources read: code.claude.com/docs/en/legal-and-compliance and code.claude.com/docs/en/agent-sdk/overview.

- Allowed, in plain words: an end user signs in to the unmodified Claude Code binary with their own subscription ("Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription"). FocusIDE runs the user's own `claude` binary and never touches credentials. Sign-in happens in Claude Code's own flow.
- The risk: the bundled adapter (`claude-agent-acp`) is built on the Claude Agent SDK. The Agent SDK page says: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK." The legal page says developers building on the Agent SDK "should use API key authentication", and may not "route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users". Anthropic "may enforce these restrictions without prior notice".
- So: a user with an API key (`ANTHROPIC_API_KEY`) or a cloud provider is clearly fine. A user on a Pro or Max subscription through FocusIDE is a grey area, and it is the user's account that would be affected.
- Branding: we may say in plain text that FocusIDE runs or works with Claude Code. We may not use Anthropic or Claude Code names or logos as our product name, in our logo, or in a way that suggests endorsement. The Agent SDK branding guide does not permit "Claude Code" as a label inside products built on the SDK; the agent picker currently says "Claude Code".
- Ways forward (owner decision): (a) ask Anthropic for written approval (contact sales, link on the legal page) before a public release; (b) release and state clearly that Claude subscription use is at the user's own risk and recommend an API key; (c) replace the Agent SDK adapter with a driver for the unmodified `claude` CLI in headless stream-json mode, which is closer to "ordinary use of Claude Code" (large change, still not confirmed by Anthropic).

### Release setup (2026-11-12)

- Decision (owner): keep Claude and follow Anthropic's rules. Changed on 2026-11-12: plan logins are now routed to the terminal instead of blocked, see "Agent terms (2026-11-12)" below. Before a Claude session starts, FocusIDE runs the user's own `claude auth status --json` and reads only the method and provider. A Claude plan login (or no login) does not start; the app explains how to set `ANTHROPIC_API_KEY` or a cloud provider. API key and Bedrock/Vertex/Foundry start as normal. Checked on this machine: a Max login is blocked, the same with a test key is allowed. The agent is labelled "Claude" (Agent SDK branding guide).
- Auto-update: `tauri-plugin-updater` with signed `latest.json` from GitHub Releases. It checks 15 s after start and every 6 hours, downloads in the background, and installs only when the user picks "Restart to update". Settings has "Check for updates" and an on/off switch. The signing key is outside the repo (`~/.tauri/focuside-updater.key` with its password file) and in the repo secrets.
- Issues: issue forms (bug, feature) and "Report a bug" / "Request a feature" in Settings and the palette, with the version and OS filled in. A failed agent start has "Report this problem".
- Release: pushing a `v*` tag runs CI, builds the Windows installer, signs the update and drafts a GitHub Release with `latest.json`.
- Website: `site/` deployed to GitHub Pages at https://yobotn.github.io/focuside/ (SEO, JSON-LD, llms.txt, llms-full.txt, markdown twin, AI crawler rules). focuside.dev is registered but points to the registrar's parking page. To move: point DNS to GitHub Pages, add `site/CNAME` with `focuside.dev`, set the custom domain in the repo settings, and replace `https://yobotn.github.io/focuside/` with `https://focuside.dev/` in `site/` and README.md.

### Agent terms (2026-11-12)

Research and plan: [docs/agent-terms.md](docs/agent-terms.md). Phase 0 to Phase 3 of that plan are done.

- Decision (owner): FocusIDE never has its own login for any agent and never reads, stores or proxies credentials. It only starts the user's installed agent, which runs its own login.
- Claude plan logins are routed, not blocked. Blocking conflicts with the Commercial Terms clause that bars restricting "any authentication method built into" Claude Code. `Session::start` still runs `claude auth status --json`, but a plan login (or no login) now returns a routing result (`agent::ClaudeRoute`, event `claude_route`). The UI explains that chat mode needs an API key or a cloud provider, and offers "Open terminal", where the user types `claude` and Claude Code runs with its own login. FocusIDE does not type into the terminal. Phase 2 replaced this route with a Claude pane, see "Pane mode (Phase 2, 2026-11-13)" below.
- The adapter starts with `--hide-claude-auth` (present in claude-agent-acp 0.106.0, `src/hide-claude-auth.ts`). It hides the claude.ai login method and refuses plan-billed sessions with `authRequired` and reason `claude_subscription_not_supported`; FocusIDE maps that reason to the same routing result.
- Launch data updated from the ACP registry: Codex install `npm install -g @agentclientprotocol/codex-acp` (it carries its own `@openai/codex`; the command is still `codex-acp`), `gemini --acp`, `qwen --acp --experimental-skills`.
- Phase 0 state: the installer included the `claude-agent-acp` executable, which embeds Anthropic's Claude Agent SDK. Phase 1 replaced that with a download on first use (below).
- Phase 1 (2026-11-13): the installer no longer carries `claude-agent-acp` (`externalBin` removed from `tauri.bundle.conf.json`, the release job no longer runs `fetch-adapter.mjs`). `crates/core/src/adapter_download.rs` pins v0.106.0 and one SHA-256 per release archive (the six targets of `fetch-adapter.mjs`; the hashes match GitHub's asset digests). When a Claude session cannot start because the adapter is missing, the "Before you start" banner names the source (GitHub, agentclientprotocol/claude-agent-acp, the version) and the size (about 185 to 220 MB, the archive includes the Claude CLI), with a Download button and a percent. The app streams the archive to `<app data>/adapters/.download`, checks the hash, extracts only `claude-agent-acp[.exe]` (the Claude CLI is never written) to `<app data>/adapters/claude-agent-acp-<version>/`, removes other versions, then starts the waiting session. `locate_adapter()` checks `FOCUSIDE_ACP_ADAPTER`, then app data, then the old sidecar places, so `npm run dev` with `fetch-adapter.mjs` still works (the script now checks the same hashes). HTTP uses reqwest with rustls and ring, the same setup as the updater. `scripts/check-bundle.mjs` fails when a bundle config (externalBin or resources) or a build output folder would carry `claude`, `claude.exe` or `claude-agent-acp*`; it runs in `npm run lint`, CI, and before and after the release build. Checked on this machine: a live download of the Windows x64 archive (219 MB) installed only `claude-agent-acp.exe` (118.5 MB). `THIRD_PARTY_NOTICES.md` stays in the bundle and now says the adapter is downloaded from upstream, not redistributed. Phase 1 does not change the Commercial Terms question (docs/agent-terms.md).
- The "Claude Code connector" string is now "Claude connector", and the picker fallback label is "Claude" (Agent SDK branding guide).
- Not done in Phase 0: showing the adapter's own auth methods. The adapter lists terminal login methods only when the client declares terminal auth support, and running one needs a PTY that starts a program, which comes with Phase 2.

### Pane mode (Phase 2, 2026-11-13)

Phase 2 of [docs/agent-terms.md](docs/agent-terms.md). A session can run in pane mode: the agent's own unmodified TUI (for example `claude`) runs in a PTY, drawn with xterm.js in the center view. The user types every prompt there. FocusIDE records turns around it: snapshots, the changes card and Undo turn.

| Decision | Why |
|---|---|
| ACP for chat mode; the agent's unmodified TUI in a PTY for pane mode. Still no stream-json and no `-p` | Replaces "ACP only, Claude Code only in v1" above. Pane mode is the path Anthropic's text describes for plan logins ("signing in to the unmodified Claude Code binary") |
| Claude plan logins and signed-out Claude open a Claude pane (`agent::ClaudeRoute::Pane`, which replaces the Phase 0 `Terminal` route). A new `claude_route` command runs `claude auth status --json` once per app run; new Claude sessions use its answer. A chat session that is routed later shows "Open Claude pane" | Plan users go straight to the pane and never need the adapter download. API key and cloud users keep chat mode. Other agents default to chat |
| Any installed agent with a TUI can open in a pane: palette "Open <agent> in a pane", and the new session menu (the caret next to "+") | Pane mode is not Claude only |
| Pane sessions put the TUI in the center. The changes cards sit beside it when the center is 980 px or wider, above it otherwise. No message box, no conversation | Deviation from "the conversation is the center" (sections 1 and 5). The TUI is the conversation in this mode |
| No new Settings row | "Settings max 6 rows". Mode comes from the login and the agent, and the new session menu picks it per session |
| Turn boundaries without hooks: a turn starts when the pane opens and right after each Checkpoint; it ends on Checkpoint, pane exit and pane close. `TurnRecorder` (`turn_recorder.rs`) does claim, start snapshot, watch log, end snapshot, diff, record, release and prune for chat sessions and panes alike; `Pane::begin` and `Pane::end_turn` are what Phase 3 hooks will call | The agent-terms plan. Hooks are Phase 3 |
| A pane holds the project claim while its turn is open, so while a pane runs, a chat turn in the same project gets "Another session is running here." When another session holds the claim, the pane shows "This turn is not tracked because another session was running" and Checkpoint tries again. Undo and redo with a running pane end the pane's open turn, run, then start a new one | Keeps "one running turn per project" correct without hooks. A pane never blocks undo |
| "Includes changes from outside this session" is never set for panes (`turn_recorder::outside_changes` needs tool information) | A pane reports no tool calls, so every change would look like an outside change |
| FocusIDE never types into a pane. `pty_write` carries only xterm.js input (keys, paste, and xterm's own answers to terminal queries). "Run in terminal" stays in the shell drawer. After Undo turn in a pane, the pane shows "You undid this turn. Claude does not know this." instead of the chat mode's prompt note | Automated driving risk (Consumer Terms section 3) |
| Launch: the program is found with the same lookup as chat mode (`which::find`, npm shims followed, `node <script>` for scripts; `locate_claude` for Claude). The PTY adds only `TERM` and `COLORTERM`; no variable is added or removed for auth. Claude starts with no arguments. A test fails if any TUI argument is `-p`, `--print`, `--bare`, `--append-system-prompt`, `--system-prompt` or `--settings` | Forced login method and classifier risks |
| `pty_open` takes a session key instead of a program from the UI. With a key it starts that pane's launch, resolved in Rust (`Pane::launch`); `pty::spawn` takes program, args and extra env. PTYs carry their session, and closing a pane session kills its PTY tree | The UI cannot choose what runs in an agent pane |
| PTY exit is taken from the process (`child.wait`), then the reader gets 400 ms to drain | ConPTY keeps the output pipe open after the program exits until the pseudo console closes, so reader EOF alone missed exits |
| A saved pane session (`SavedSession.mode = "pane"`, with a FocusIDE id `pane-<key>` in `acpId`) reopens as a fresh TUI. No resume flag is passed | Resume flags differ per agent and were not checked |
| The plan card and the turn summary are not shown in panes. The cards column says they need hooks | They come from hook data. Phase 3 adds them, see "Hooks (Phase 3, 2026-11-13)" below |
| Product labels say "Claude" ("Claude pane", "Open Claude pane"). The pane note says "Claude Code runs with its own login. Billing and limits follow your Claude plan and Anthropic's terms."; other agents get a generic line | Naming rule; policy change risk |

TUI commands (`AgentSpec.tui_program`, `tui_args`):

| Agent | Command | Checked |
|---|---|---|
| Claude | `claude` | Run on this machine (Claude Code 2.1.323) |
| Codex | `codex` | From the Codex CLI docs. Not installed here |
| Gemini CLI | `gemini` | From the Gemini CLI docs. Not installed here |
| OpenCode | `opencode` | From the OpenCode docs. Not installed here |
| Qwen Code | `qwen` | Unverified (Gemini CLI fork). Not installed here |
| Goose | `goose session` | Goose docs: "Start or resume interactive chat sessions". Not installed here |

Verified on this machine (Windows 11):
- `cargo test`: 4 new `turn_recorder` tests (record, release and undo of a turn; a continued turn updates its record and adds commands; a failed snapshot releases the claim; the outside rule with and without tool information), the PTY test (`cmd /c echo` with an extra variable through `pty::spawn`, ConPTY, output and exit), and the TUI flag test. Chat sessions use `TurnRecorder` with the same events and fields as before.
- Live, ignored tests: `claude --version` through `pty::spawn` printed "2.1.323 (Claude Code)". The full `claude` TUI started in a PTY in an empty folder, drew its screen, and ended when the PTY closed (`cargo test -p focuside-desktop -- --ignored real_claude`). Nothing was typed into it.
- ConPTY sends a cursor position request (`ESC [6n`) at start and the program waits for the answer. xterm.js answers it in the app; the tests answer it the same way. Pane output that arrives before `pty_open` returns is kept and replayed, so this request is not lost.
- `npm run lint`, `svelte-check` and `vite build` pass.

Not verified:
- The app itself was not run. Not checked in the window: AgentPane drawing, Checkpoint and the changes cards, Undo turn from a pane, switching sessions while a TUI keeps running, the new session menu, and the Claude route at startup.
- The ConPTY test list from docs/agent-terms.md: resize, alt screen, mouse and paste of large text in the Claude TUI.
- Codex, Gemini CLI, OpenCode, Qwen Code and Goose TUIs (not installed). macOS and Linux (CI only).
- Keys: like the shell drawer, a focused pane keeps its keys, so Mod+K and other app keys do not work while the pane has focus. Click outside the pane first.

Not done in Phase 2: hooks (done in Phase 3, below), and showing the adapter's own auth methods in a pane. Promoting pane mode to Claude plan users still waits on Anthropic's answers to questions (1), (2) and (5) in docs/agent-terms.md.

### Hooks (Phase 3, 2026-11-13)

Phase 3 of [docs/agent-terms.md](docs/agent-terms.md): observe-only hooks for panes. Checked against the live docs on 2026-11-13: Claude Code [hooks](https://code.claude.com/docs/en/hooks), [plugins](https://code.claude.com/docs/en/plugins), [plugin manifest reference](https://code.claude.com/docs/en/plugins-reference), [CLI reference](https://code.claude.com/docs/en/cli-reference), [Codex hooks](https://learn.chatgpt.com/docs/hooks), [Gemini CLI hooks](https://geminicli.com/docs/hooks/).

| Decision | Why |
|---|---|
| Helper `focuside-hook` (`crates/hook`): reads the hook JSON on stdin and writes one JSON file per event into `$FOCUSIDE_EVENTS` (temporary name, then rename). It prints nothing to stdout or stderr, always exits 0 (a silent panic hook and `catch_unwind`), reads no other environment variable, exits at once when `FOCUSIDE_EVENTS` is missing, and never reads `transcript_path`. Stored fields: schema version `v: 1`, agent, event, session id, tool name, file path, command, plan, last message, StopFailure error. The prompt is not stored | Hook stdout on UserPromptSubmit becomes model context, exit 2 blocks tools, and the hook inherits API keys. Silence and exit 0 keep it observe-only |
| The helper ships as a Tauri sidecar (`bundle.externalBin: binaries/focuside-hook` in `tauri.bundle.conf.json`). `scripts/build-hook.mjs --release` builds it and copies it to `src-tauri/binaries/focuside-hook-<triple>` from the bundle config's `beforeBuildCommand`, so `npm run build` and the release workflow both get it. `npm run dev` builds the debug helper next to the dev app (`predev`). At run time FocusIDE looks next to its own executable | The plugin must find a real executable on every OS. `check-bundle.mjs` still passes; it only forbids `claude` and `claude-agent-acp` |
| Delivery Option A only: FocusIDE writes the plugin `focuside` into `<app data>/claude-plugin/focuside/` for the current OS (`.claude-plugin/plugin.json`, `hooks/hooks.json`, `hook/focuside-hook[.exe]`) and starts Claude panes with `--plugin-dir <that folder>`. No marketplace, nothing in `~/.claude`, nothing in the project | Plugin and app versions always match. The flag loads a plugin "for this session only" |
| `hooks/hooks.json` wraps the events in a top-level `"hooks"` key and uses exec form (`command` plus `args`, `${CLAUDE_PLUGIN_ROOT}/hook/focuside-hook.exe`, `args: ["claude"]`, timeout 10 s). Events: UserPromptSubmit, Stop, StopFailure, PreToolUse (matcher `ExitPlanMode`), PostToolUse (matcher `ExitPlanMode\|Bash\|PowerShell\|Edit\|Write\|MultiEdit\|NotebookEdit`). No SessionStart, no UserPromptExpansion, no skills, agents, output style or MCP, no `async`, `asyncRewake` or `additionalContext`. A test checks this | Exec form spawns the helper with no shell, so paths with spaces work and bash and PowerShell differences do not matter. The docs say exec form on Windows needs a real `.exe` |
| The helper sits in `hook/`, not `bin/` | The plugin docs: files in a plugin's `bin/` are on the PATH of Claude's Bash tool, so Claude could see and run them |
| Consent: before the first Claude pane FocusIDE asks "Let FocusIDE add an observe-only plugin to Claude panes in FocusIDE? It only reports when a turn starts and ends. It does not change your Claude settings." The answer is the `claudeHooks` setting. Settings has a Hooks section below the rows with the switch | Adding a launch flag needs consent. The section sits below the rows, like "About", so "Settings max 6 rows" still holds |
| Every pane gets `FOCUSIDE_EVENTS=<app data>/projects/<id>/events/<session>`, also without consent. The folder is emptied when the pane starts and removed when it closes. A notify watcher per pane (50 ms debounce) reads the files in name order, deletes them and hands them to the pane one at a time. Events whose agent is not the pane's agent are ignored | The variable alone does nothing. It lets hooks the user added for Codex or Gemini CLI report turns. Nothing is written to the user's repo |
| Turns with hooks: UserPromptSubmit ends any open turn and starts one; Stop and StopFailure end it. The pane holds the project claim only in between. The next UserPromptSubmit (Stop does not fire after an interrupt), Checkpoint, pane exit and pane close also release it. A prompt that arrives while another session holds the claim records no turn and shows "This turn was not tracked because another session was running." | Fixes the Phase 2 claim problem: an idle pane no longer blocks chat turns in the project |
| A pane started with the plugin starts no turn when it opens. Until the first hook event arrives, Checkpoint starts and ends turns by hand, as in Phase 2. The status says "Turn tracking starts with your first prompt. If no turn shows up, hooks did not run. Then use Checkpoint." Panes without the plugin keep the Phase 2 rules until a hook event arrives | FocusIDE cannot tell that hooks are blocked (`disableAllHooks`, `allowManagedHooksOnly`, workspace trust) until a prompt shows no event |
| Undo or redo while a hook turn runs waits, like a chat turn ("Wait for the running turn to finish. In a pane, Checkpoint ends it."). A turn started by hand is ended and started again around the undo, as in Phase 2 | A hook turn belongs to a running prompt |
| Hook turns pass PostToolUse file paths and commands to `TurnRecorder` as edits and commands, so the turn record, the undo dialog and "Includes changes from outside this session" work as in chat mode | Same rule as ACP tool calls |
| Plan card in panes: ExitPlanMode `tool_input.plan` (PreToolUse) shows a read-only card, "proposed"; `tool_response.plan` (PostToolUse) marks it "approved". It uses the same parser as chat mode (`plan.rs`, fallback for free-form plans). The changes card of that turn marks files that are not in the approved plan. Approval stays in the TUI | FocusIDE never answers for the user in a pane |
| Turn summary without a model call (`crates/core/src/summary.rs`): files, new and deleted files, lines, commands run (first 5, no repeats), plan step count and the first paragraph of Stop's `last_assistant_message` (280 characters). It is in every `turn_ended` event and shows in the changes card for chat and pane turns. Chat turns have no message excerpt: the conversation already shows the answer | No second model call and no `claude -p` |
| Codex and Gemini CLI snippets in Settings with Copy, built in Rust with the helper's path for this OS (`hooks.rs`). FocusIDE never writes `~/.codex` or `~/.gemini` | Each calls the same helper with the agent name and prints nothing |

Codex snippet (`~/.codex/hooks.json`): UserPromptSubmit, Stop, PreToolUse and PostToolUse (matcher `*`), `type: "command"`, `timeout: 10`, `command` and, on Windows, `commandWindows` with the quoted helper path plus `codex`. From the live Codex docs: hooks are on by default, `commandWindows` is "an optional Windows-only command override", UserPromptSubmit plain stdout "is added as extra developer context", and "Exit 0 with no output is treated as success". Gemini CLI snippet (the `hooks` key of `~/.gemini/settings.json`): BeforeAgent, AfterAgent, BeforeTool and AfterTool (matcher `*`), `timeout: 10000` (milliseconds), command `& '<helper>' gemini` on Windows (Gemini runs PowerShell there) and `'<helper>' gemini` elsewhere. The helper maps Gemini's names to Claude's (BeforeAgent to UserPromptSubmit, AfterAgent with `prompt_response` to Stop, BeforeTool and AfterTool to PreToolUse and PostToolUse).

Verified on this machine (Windows 11, Claude Code 2.1.323):
- `cargo test`: the helper's integration tests run the built binary with a cleared environment for 11 sample events (Claude, Codex, Gemini) and 9 bad inputs, with the variable missing and with a folder that cannot be created: stdout and stderr are empty and the exit code is 0 every time. Unit tests for the stored fields, Gemini names, cutting long text and the agent argument. Pane tests (with a real shadow repo): the claim is held only between UserPromptSubmit and Stop, a prompt during another session's turn is not tracked, the next prompt and Checkpoint close a turn without Stop, StopFailure ends a turn, pane exit and close release the claim, close removes the events folder, Phase 2 rules without hooks, the Checkpoint fallback when hooks never run, events from another agent are ignored, edits, commands, plan steps and the last message reach the record and the summary. Plugin tests: only the five events, exec form, no `bin/`, name rules. Snippet tests: both parse as JSON and quote paths with spaces and quotes.
- `claude --help` lists `--plugin-dir <path>` ("Load a plugin from a directory or .zip for this session only").
- Live, ignored tests (`cargo test -p focuside-desktop -- --ignored real_claude`, after `node scripts/build-hook.mjs`): `claude plugin validate <generated folder> --strict` passes (its plugin catalog checks note a missing README and icon, which only matter for a catalog submission). `claude --plugin-dir <folder> plugin details focuside@inline` lists "Hooks (5) PostToolUse, PreToolUse, Stop, StopFailure, UserPromptSubmit (harness-only, no model context cost)", 0 skills, 0 agents, 0 MCP servers, "Always-on: ~0 tok". `plugin list --json` with the flag shows `focuside@inline`, scope `session`. The Claude TUI starts in a PTY with `--plugin-dir` and `FOCUSIDE_EVENTS`, draws its screen and closes; no event is written because no prompt is typed. The helper copy in the plugin folder writes a Stop event that the pane's reader parses.
- `node scripts/build-hook.mjs --release` builds a 237 KB helper into `src-tauri/binaries`. `npm run lint`, `npm test`, `svelte-check` and `vite build` pass.

Not verified:
- Hooks firing in a real Claude session. That needs a typed prompt, and nothing typed one (FocusIDE never types into an agent PTY). So the UserPromptSubmit, Stop, StopFailure and ExitPlanMode payloads and the `${CLAUDE_PLUGIN_ROOT}` substitution in exec form on Windows come from the docs only. Next step: open a Claude pane in the app, type a prompt by hand and check the changes card.
- Whether workspace trust holds back `--plugin-dir` hooks (the docs only say so for repository plugins). Managed `disableSideloadFlags` makes Claude exit 1 when `--plugin-dir` is passed; FocusIDE does not detect that case, so the pane exits and the user has to turn the plugin off in Settings. `allowManagedHooksOnly` blocks the plugin's hooks; the pane then works with Checkpoint.
- Codex and Gemini CLI (not installed). Unverified: Codex `commandWindows` quoting through `cmd /C`; Codex and Gemini tool names for file edits (only tools with `tool_input.command`, `file_path` or `path` are read); Gemini treating empty stdout as no output (from source, the docs show `{}`); whether Gemini's environment sanitizer keeps `FOCUSIDE_EVENTS`; Codex needs the user to trust new hooks with `/hooks`.
- The app window was not run: the plugin question, the Settings Hooks section, the pane plan card and the summary rows were checked by svelte-check and vite build only.
- macOS and Linux (CI only): the sidecar next to the executable and its exec bit. The release workflow has not run with the new `beforeBuildCommand`.

Not done in Phase 3: delivery Option B (marketplace plugin); the "Update the focuside plugin" notice (Option A ships plugin and app together, so FocusIDE does not check `v` yet); keeping the summary in `turns.json` (it lives in the session's cards only).

### More agents (Phase 4, 2026-11-13)

Phase 4 of [docs/agent-terms.md](docs/agent-terms.md): seven more agents, per-agent data and terms notes, and a monthly registry check. Each launch command was checked on 2026-11-13 against the [ACP registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json) and the vendor docs. None of the new agents is installed on this machine, so none was run.

| Agent | Chat mode | Pane | Install hint | Checked against | Status |
|---|---|---|---|---|---|
| Antigravity CLI | `agy acp` | `agy` | antigravity.google/docs/install (script installer) | [Antigravity CLI ACP](https://antigravity.google/docs/cli/acp.md) ("agy acp [flags]", sign in with `agy` or `agy acp --login`). The registry entry `antigravity-acp` is a different program, `agy_acp_server` from the IDE extensions | Verified in docs; registry differs on purpose |
| Copilot CLI | `copilot --acp` | `copilot` | `npm install -g @github/copilot` | Registry `github-copilot-cli`, [GitHub ACP server reference](https://docs.github.com/en/copilot/reference/acp-server) (public preview, stdio default), npm bin `copilot`, GitHub install page | Verified |
| Cursor CLI | `cursor-agent acp` | `cursor-agent` | cursor.com/docs/cli/installation (script installer) | Registry `cursor` (`cursor-agent acp`), [Cursor ACP](https://cursor.com/docs/cli/acp) (`agent acp`), cursor.com/install (creates `agent` and `cursor-agent`) | Verified. Unverified: starting the Windows `cursor-agent.cmd`, which runs a PowerShell script, as a child process and in a PTY |
| Kiro CLI | `kiro-cli acp` | `kiro-cli` | kiro.dev/docs/getting-started/installation/ | [Kiro ACP](https://kiro.dev/docs/cli/acp/), [Kiro CLI setup](https://kiro.dev/docs/cli/setup/) ("run `kiro-cli`. The terminal UI opens") | Verified. The registry adds `--agent-engine=v3`, which overrides the user's saved setting, so FocusIDE leaves it out. The registry has no Windows build; Kiro lists Windows 11. Windows launch unverified |
| Cline | `cline --acp` | `cline` | `npm install -g cline` | Registry `cline`, [Cline ACP docs](https://docs.cline.bot/cline-cli/acp-editor-integrations), Cline CLI overview (`cline` with no arguments is interactive), npm bin `cline` | Verified |
| Kimi Code | `kimi acp` | `kimi` | `npm install -g @moonshot-ai/kimi-code` | Registry `kimi-code` (npx package with `acp`), npm bin `kimi`, [Kimi Code site](https://moonshotai.github.io/kimi-code/) (`kimi` starts a conversation) | ACP flag verified in the registry only; the vendor site does not document ACP. The legacy Kimi CLI used the same `kimi` name |
| Mistral Vibe | `vibe-acp` | `vibe` | `uv tool install mistral-vibe` | Registry `mistral-vibe`, [README](https://github.com/mistralai/mistral-vibe) and `pyproject.toml` (scripts `vibe` and `vibe-acp`) | Verified |

| Decision | Why |
|---|---|
| Install hints are plain commands or the vendor's install page. Script installers (a curl or irm command piped to a shell) are not copied into the app | FocusIDE never installs an agent. The page shows the right script for each OS |
| Antigravity CLI opens in a pane by default. Chat mode shows a one-time note: "Google's terms on third-party apps are unclear for personal Google accounts. An API key or an enterprise login is the safer path." The user picks "Use chat mode" or "Open a pane instead". The answer is kept in `settings.chatConfirmed` | The report says consumer logins in chat mode wait for Google's written answer. FocusIDE cannot read the login, so it cannot gate on it; the user decides with the facts in front of them |
| Gemini CLI is unchanged; its note says Login with Google stopped working for personal accounts on June 18, 2026 | Confirmed in docs/agent-terms.md |
| One data or terms line per agent in Settings (under Default agent) and in the new session menu and palette (as the item detail). Lines exist only for facts marked Confirmed in "Data use per auth mode": Claude, Copilot, Cursor, Kiro, Gemini CLI, Antigravity (terms only). OpenCode, Goose, Qwen Code, Cline, Kimi Code and Mistral Vibe share "This agent uses the provider you set up in it. That provider's terms apply." Codex has no line (its data use is Unverified) | Users cannot see training defaults from FocusIDE, and FocusIDE must not read their settings |
| No Antigravity data-use line | "Data use per auth mode" has no Confirmed row for Antigravity |
| `scripts/check-agents.mjs` compares every `AgentSpec` with the registry and prints differences. Known differences (Claude adapter, Antigravity, Kiro) are listed in the script. `--strict` exits 1 on an unknown difference. The "Agent commands" workflow runs it monthly and on demand and only reports; CI does not run it on push (network) | Vendor CLIs update on their own |
| Windows npm shims that point at an `.mjs` file or an extensionless node script now resolve to `node <script>` (`which::is_node_script`) | Kimi Code's bin is `dist/main.mjs` and Cline's is `bin/cline` with a node shebang; before this, both stayed as `.cmd` paths |

Not done in Phase 4: running any new agent live on Windows (none installed); hook names for Antigravity, Copilot, Cursor and Kiro panes (they use Checkpoint); Cursor's `cursor/create_plan` and `cursor/update_todos` ACP extensions in the plan card; a "Preview" badge for Copilot beyond its note line; reading versions or install hints from the registry at run time.

## 11. Open items

1. Owner says go to create the public repo `yobotn/focuside` and push.
2. Set up the focuside.app redirect.
3. Trademark search for FocusIDE.
4. ~~Check Anthropic's policy~~ done 2026-11-12, see section 10. Decide between ways (a), (b) and (c) before the public release. Update 2026-11-12: see "Agent terms (2026-11-12)" and the questions for Anthropic in docs/agent-terms.md.
5. ~~Which agent comes second in v2~~: all known ACP agents are now selectable; check each one live once installed.
6. Rename the local folder from `terminalScreen` to `focuside` (optional, before the first push).
