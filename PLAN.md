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
- v2 adds Codex (`codex-acp`) and Gemini (`gemini --acp`, check the move to Antigravity CLI first).

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
| Claude subscription use through the Agent SDK (which the ACP adapter uses) counts against a separate Agent SDK credit, and third-party apps may not offer Claude.ai login without approval | FocusIDE never handles credentials, it only starts the user's own `claude`. Check Anthropic's policy before public release. Claude in the terminal drawer stays available |
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

- **More agents (pulled forward from v2).** Settings > Default agent lists Claude Code, Codex (`codex-acp`), Gemini CLI (`gemini --experimental-acp`), OpenCode (`opencode acp`), Qwen Code (`qwen --experimental-acp`), Goose (`goose acp`) and Custom command. Each one shows "(not installed)" when it is not on PATH. The palette has "New session with <agent>" for each installed agent. A custom command can start with `NAME=value` parts, which set environment variables. Sessions remember their agent, so resume uses the same one. "Plan first" is turned off for agents that do not report a plan mode. Snapshots, undo and the changes card do not depend on the agent.
  - Checked live: the picker, a custom command (the spawn path ran and the agent's own error came back), the agent name in the bar and in errors.
  - Not checked live: Codex, Gemini, OpenCode, Qwen and Goose themselves (none are installed on this machine). The `--experimental-acp` flags may change upstream.
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

- Decision (owner): keep Claude and follow Anthropic's rules. Before a Claude session starts, FocusIDE runs the user's own `claude auth status --json` and reads only the method and provider. A Claude plan login (or no login) does not start; the app explains how to set `ANTHROPIC_API_KEY` or a cloud provider. API key and Bedrock/Vertex/Foundry start as normal. Checked on this machine: a Max login is blocked, the same with a test key is allowed. The agent is labelled "Claude" (Agent SDK branding guide).
- Auto-update: `tauri-plugin-updater` with signed `latest.json` from GitHub Releases. It checks 15 s after start and every 6 hours, downloads in the background, and installs only when the user picks "Restart to update". Settings has "Check for updates" and an on/off switch. The signing key is outside the repo (`~/.tauri/focuside-updater.key` with its password file) and in the repo secrets.
- Issues: issue forms (bug, feature) and "Report a bug" / "Request a feature" in Settings and the palette, with the version and OS filled in. A failed agent start has "Report this problem".
- Release: pushing a `v*` tag runs CI, builds the Windows installer, signs the update and drafts a GitHub Release with `latest.json`.
- Website: `site/` deployed to GitHub Pages at https://yobotn.github.io/focuside/ (SEO, JSON-LD, llms.txt, llms-full.txt, markdown twin, AI crawler rules). focuside.dev is registered but points to the registrar's parking page. To move: point DNS to GitHub Pages, add `site/CNAME` with `focuside.dev`, set the custom domain in the repo settings, and replace `https://yobotn.github.io/focuside/` with `https://focuside.dev/` in `site/` and README.md.

## 11. Open items

1. Owner says go to create the public repo `yobotn/focuside` and push.
2. Set up the focuside.app redirect.
3. Trademark search for FocusIDE.
4. ~~Check Anthropic's policy~~ done 2026-11-12, see section 10. Decide between ways (a), (b) and (c) before the public release.
5. ~~Which agent comes second in v2~~: all known ACP agents are now selectable; check each one live once installed.
6. Rename the local folder from `terminalScreen` to `focuside` (optional, before the first push).
