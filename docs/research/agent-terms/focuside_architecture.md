# FocusIDE architecture today, and where per-agent integration modes would land

Scope: a map of the FocusIDE repo at commit 3cfa442 (branch main, clean tree), read on 2026-11-12. Every source is a repo file, cited as path:line. Paths are relative to the repo root `C:/Users/YB/Desktop/focuside.dev`. No existing file was modified. Note on format: the repo text lint (`scripts/text-lint.mjs:9-12`) fails on em dashes and emojis in any .md file under the repo (it walks the whole tree, `scripts/text-lint.mjs:7-8,14-19`), so this file uses " - " as the separator between a finding and its source.

## Q1. How agents are discovered and configured, which agents exist, how the claude-agent-acp adapter is bundled and launched, and what auth guidance users get

### Takeaway
Every agent today is an ACP agent over stdio JSON-RPC. Claude goes through a bundled, pinned `claude-agent-acp` sidecar (v0.106.0) that runs the user's own `claude` via `CLAUDE_CODE_EXECUTABLE`; five other agents (Codex, Gemini CLI, OpenCode, Qwen Code, Goose) plus a free-form custom command are found on PATH. Before a Claude session starts, FocusIDE runs `claude auth status --json` and refuses to start when the login is a Claude.ai subscription or signed out, and the UI, README and llms.txt all tell users to use an API key or a cloud provider.

### Cited Findings

Agent table and lookup
- The agent list is a static `AGENTS: &[AgentSpec]` table with fields `id`, `name`, `program`, `args`, `install`. Entries: `claude` (program `claude`), `codex` (`codex-acp`), `gemini` (`gemini --experimental-acp`), `opencode` (`opencode acp`), `qwen` (`qwen --experimental-acp`), `goose` (`goose acp`) - [crates/core/src/agent.rs:184-239](crates/core/src/agent.rs)
- `AgentSpec` has no field for an integration mode, a TUI command, or auth requirements; it only describes how to start an ACP process - [crates/core/src/agent.rs:186-194](crates/core/src/agent.rs)
- `agents()` reports each agent with an optional resolved path (installed or not). For `claude` it uses `locate_claude()`, for others `which::find(program)` - [crates/core/src/agent.rs:241-265](crates/core/src/agent.rs)
- `agent_command(id, custom)` builds an `AgentCommand`. `claude` or empty id goes to `claude_command()`. `custom` splits the user's command line; leading `NAME=value` parts become env vars. Other ids use the spec's program and args. npm `.js` shims are run as `node <script>` - [crates/core/src/agent.rs:267-351](crates/core/src/agent.rs)
- `AgentCommand` is `{ adapter: PathBuf, args, claude: Option<PathBuf>, env }`; `claude` is passed to the child as `CLAUDE_CODE_EXECUTABLE` - [crates/core/src/acp.rs:213-223](crates/core/src/acp.rs), [crates/core/src/acp.rs:240-247](crates/core/src/acp.rs)
- `locate_claude()` honors `FOCUSIDE_CLAUDE`, else finds `claude` on PATH and follows npm shims - [crates/core/src/agent.rs:67-73](crates/core/src/agent.rs)
- Tauri command `agent_list` exposes the table to the UI; `session_start(project_id, resume, agent_id, custom_command)` maps missing parts to `missing:claude`, `missing:adapter`, `missing:agent` - [apps/desktop/src-tauri/src/commands.rs:57-63](apps/desktop/src-tauri/src/commands.rs), [apps/desktop/src-tauri/src/commands.rs:162-202](apps/desktop/src-tauri/src/commands.rs)
- UI settings store `agent` (default `"claude"`) and `agentCommand` (custom line) - [apps/desktop/src/lib/stores/app.svelte.ts:14-25](apps/desktop/src/lib/stores/app.svelte.ts), [apps/desktop/src/lib/stores/app.svelte.ts:59](apps/desktop/src/lib/stores/app.svelte.ts)
- `newSession` picks `resume?.agent ?? agentId ?? settings.agent` and calls `sessionStart`; the saved session remembers its agent so resume uses the same one - [apps/desktop/src/lib/stores/app.svelte.ts:293-332](apps/desktop/src/lib/stores/app.svelte.ts), [apps/desktop/src/lib/stores/session.svelte.ts:59-67](apps/desktop/src/lib/stores/session.svelte.ts)
- Settings shows one "Default agent" select with each agent, "(not installed)" when missing, and "Custom command" with a text field - [apps/desktop/src/lib/components/Settings.svelte:77-103](apps/desktop/src/lib/components/Settings.svelte)
- PLAN.md records that other agents were pulled forward from v2 after owner feedback, with "Plan first" off for agents without a plan mode, and that Codex, Gemini, OpenCode, Qwen and Goose were not checked live - [PLAN.md:512-516](PLAN.md)
- "Plan first" is disabled when the agent reports no `plan` mode in `modes.availableModes` or the `mode` config option - [apps/desktop/src/lib/stores/session.svelte.ts:209-220](apps/desktop/src/lib/stores/session.svelte.ts), [apps/desktop/src/lib/components/MessageBox.svelte:183-184](apps/desktop/src/lib/components/MessageBox.svelte)

Adapter bundling and launch
- `scripts/fetch-adapter.mjs` pins `ADAPTER_VERSION = "0.106.0"`, downloads `claude-agent-acp-<os>.zip|tar.gz` from `github.com/agentclientprotocol/claude-agent-acp/releases`, and saves it as `apps/desktop/src-tauri/binaries/claude-agent-acp-<target triple>[.exe]` - [scripts/fetch-adapter.mjs:10-39](scripts/fetch-adapter.mjs), [scripts/fetch-adapter.mjs:71](scripts/fetch-adapter.mjs)
- The release archive ships its own Claude CLI next to the adapter; the script deletes it and keeps only the adapter, because FocusIDE points the adapter at the user's own `claude` - [scripts/fetch-adapter.mjs:73-75](scripts/fetch-adapter.mjs)
- The binary is a Tauri sidecar via `bundle.externalBin: ["binaries/claude-agent-acp"]` in the bundle config used by `tauri:build` - [apps/desktop/src-tauri/tauri.bundle.conf.json:1-6](apps/desktop/src-tauri/tauri.bundle.conf.json), [apps/desktop/package.json](apps/desktop/package.json) (script `tauri:build`)
- `locate_adapter()` order: `FOCUSIDE_ACP_ADAPTER`, then next to the app exe (`claude-agent-acp[.exe]` or the triple-suffixed name), then walks up to `apps/desktop/src-tauri/binaries` for dev - [crates/core/src/agent.rs:39-65](crates/core/src/agent.rs)
- The adapter exe is about 113 MB on Windows - [PLAN.md:463](PLAN.md)
- `AgentProcess::spawn` starts the adapter in the project root inside a `ProcessTree` (Job Object on Windows), with piped stdio; the last 40 stderr lines are kept for errors - [crates/core/src/acp.rs:234-285](crates/core/src/acp.rs)
- The ACP client is a hand-written JSON-RPC client on serde_json (one message per line), not the SDK crate - [crates/core/src/acp.rs:1-4](crates/core/src/acp.rs), [PLAN.md:444](PLAN.md), [PLAN.md:462](PLAN.md)
- `initialize` advertises `fs.readTextFile=false`, `fs.writeTextFile=false`, `terminal=false`: the agent reads and writes the disk itself and runs its own commands - [crates/core/src/acp.rs:287-298](crates/core/src/acp.rs)
- Capabilities read: `loadSession`, `resume`, `list`, `image`, `systemPrompt`, `authMethods` - [crates/core/src/acp.rs:300-325](crates/core/src/acp.rs)

Auth gate and guidance
- `claude_auth()` runs the user's `claude auth status --json` (20 s timeout) with the agent env and reads only `loggedIn`, `authMethod`, `apiProvider` - [crates/core/src/agent.rs:87-150](crates/core/src/agent.rs)
- `ClaudeAuth::access()` returns `Subscription` when first party (or no provider) and the method contains `claude.ai` or `oauth` or is empty; `SignedOut` when not logged in; else `Allowed` (API key, bedrock, vertex, foundry) - [crates/core/src/agent.rs:99-133](crates/core/src/agent.rs), tests [crates/core/src/agent.rs:379-399](crates/core/src/agent.rs)
- The gate runs in `Session::start` only when `cmd.claude` is set (that is, only for the Claude adapter path). On `Subscription` or `SignedOut` it emits `claude_access` and aborts the start - [apps/desktop/src-tauri/src/session.rs:136-148](apps/desktop/src-tauri/src/session.rs)
- Signed-out also maps to the same "claudeAccess" screen: the UI comment says signing in to a Claude account is not offered, ask for a key - [apps/desktop/src/lib/stores/session.svelte.ts:225-233](apps/desktop/src/lib/stores/session.svelte.ts)
- The `ClaudeAccess.svelte` panel explains the rule, links to the Console key page, shows `setx ANTHROPIC_API_KEY "sk-ant-..."` on Windows or a `~/.claude/settings.json` env snippet on Unix, offers cloud provider docs, "Check again" and "Use another agent" - [apps/desktop/src/lib/components/ClaudeAccess.svelte:1-60](apps/desktop/src/lib/components/ClaudeAccess.svelte)
- UI strings: "Claude needs an API key in FocusIDE"; "Anthropic does not allow apps like FocusIDE, which connect through the Claude Agent SDK, to use Claude plans. So FocusIDE starts Claude only with an API key or a cloud provider." - [apps/desktop/src/lib/i18n/en.ts:202-216](apps/desktop/src/lib/i18n/en.ts)
- README "Requirements" has an IMPORTANT box with the same rule and lists Claude Code as "Bundled `claude-agent-acp` adapter, which runs your own `claude` | API key or cloud provider. Full plan mode." - [README.md:95-121](README.md)
- `site/llms.txt` repeats: "Claude needs an API key (ANTHROPIC_API_KEY) or a cloud provider: Anthropic does not allow apps built on the Claude Agent SDK to use Claude plans" - [site/llms.txt:5](site/llms.txt)
- There is also an older first-run banner path: when Claude is installed but signed out, `EnvBanner` offers "Sign in to Claude Code" with the command `claude` and "Run in terminal" - [apps/desktop/src/lib/components/EnvBanner.svelte:14-56](apps/desktop/src/lib/components/EnvBanner.svelte), [apps/desktop/src/lib/i18n/en.ts:29-31](apps/desktop/src/lib/i18n/en.ts)
- PLAN.md policy check: the legal page allows "an end user signing in to the unmodified Claude Code binary with their own Claude subscription"; the risk is that the adapter is built on the Agent SDK, for which Anthropic does not allow claude.ai login unless approved. Three ways forward were listed: (a) ask Anthropic for approval, (b) release with user-risk warning, (c) "replace the Agent SDK adapter with a driver for the unmodified `claude` CLI in headless stream-json mode" - [PLAN.md:521-529](PLAN.md)
- Owner decision (Release setup): keep Claude via the adapter and block plan logins; label the agent "Claude" per the Agent SDK branding guide - [PLAN.md:531-533](PLAN.md)

### Inferences
- The auth gate is coupled to "Claude via adapter" by `cmd.claude.is_some()`, not to an explicit mode. A per-auth mode selector can reuse `claude_auth()` and `ClaudeAccess` unchanged and branch on it (for example: `Allowed` gives ACP mode, `Subscription` gives pane mode) instead of aborting at `session.rs:143`.
- The `ClaudeAccess` panel and the README, llms.txt and en.ts copy all assume "subscription means you cannot use Claude in FocusIDE". A pane mode for subscription users would require rewriting those strings and the README IMPORTANT box.

### Gaps
- Whether Codex, Gemini, OpenCode, Qwen and Goose actually work over ACP is unverified in the repo (PLAN.md:516 says none were installed).
- The repo has no per-agent auth detection for non-Claude agents (no equivalent of `claude auth status`).

## Q2. How a turn is defined, when snapshots are taken, how undo works, how the plan is parsed, and what the watcher does

### Takeaway
A turn is one ACP `session/prompt` request; it ends when that request returns. Snapshots go into a shadow git repo in app data at project open, at turn start (or at Run plan for planned turns), and at turn end; the changes card is `git diff start..end`. The plan card comes from the plan text inside Claude's ExitPlanMode permission request (not from files), parsed deterministically; ACP `plan` notifications only feed a live to-do line. The watcher emits `fs-changed` to the UI and logs paths during a turn to detect ignored files that changed.

### Cited Findings

Turn lifecycle (ACP mode, the only mode today)
- `Session::prompt_inner` refuses if a turn is open, starts or resumes the agent if needed, sets the ACP mode (`plan` for Plan first, `acceptEdits` for Continue, else the session's original mode), then claims the project and takes a `refs/fi/<session>/<turn>/start` snapshot unless the prompt is plan-first - [apps/desktop/src-tauri/src/session.rs:586-658](apps/desktop/src-tauri/src/session.rs)
- It prepends "The user reverted all file changes from your last turn." after an undo, stores a `TurnCtx`, emits `turn_started`, and sends `session/prompt`; the response calls `finish_turn` - [apps/desktop/src-tauri/src/session.rs:660-697](apps/desktop/src-tauri/src/session.rs)
- `finish_turn` takes the end snapshot `refs/fi/<session>/<turn>/end`, diffs start..end, adds watcher-seen ignored files, builds or updates a `TurnRecord`, releases the project, prunes in the background, and emits `turn_ended` with `changes`, `notCovered`, `commands`, `outside`, `yourEdits`, `ignoredChanged` - [apps/desktop/src-tauri/src/session.rs:699-842](apps/desktop/src-tauri/src/session.rs)
- The "outside this session" flag is set when there were no shell commands and a changed file is not in the session's tool-reported edits or the user's viewer saves - [apps/desktop/src-tauri/src/session.rs:757-762](apps/desktop/src-tauri/src/session.rs)
- During a turn, `on_update` tracks tool kinds by `toolCallId`, records edit/delete/move `locations` inside the root as `edits`, and records `execute` tool titles as `commands` - [apps/desktop/src-tauri/src/session.rs:329-389](apps/desktop/src-tauri/src/session.rs)
- One running turn per project: `Project::claim/release` with `running: Mutex<Option<String>>` - [apps/desktop/src-tauri/src/project.rs:24-94](apps/desktop/src-tauri/src/project.rs)
- Stop/Pause sends `session/cancel` and answers open permissions as cancelled, then kills the process tree after 3 s if the turn is still open - [apps/desktop/src-tauri/src/session.rs:23-24](apps/desktop/src-tauri/src/session.rs), [apps/desktop/src-tauri/src/session.rs:844-877](apps/desktop/src-tauri/src/session.rs)
- `TurnRecord` fields: `id`, `session_key`, `start`, `end`, `changes`, `not_covered`, `commands`, `edits`, `your_edits`, `ignored_changed`, `undone`, `created`; persisted to `<appdata>/projects/<id>/turns.json`, last 50 kept - [apps/desktop/src-tauri/src/turns.rs:11-65](apps/desktop/src-tauri/src/turns.rs)

Plan-first turns and plan parsing
- In a plan-first prompt the `TurnCtx.start` is None (planning is read-only). When the plan request arrives and the user picks Run plan, `run_plan` claims the project, takes the start snapshot, starts the watch log, answers with the run-plan option and may queue a switch to `acceptEdits` after `current_mode_update` - [apps/desktop/src-tauri/src/session.rs:462-521](apps/desktop/src-tauri/src/session.rs), [apps/desktop/src-tauri/src/session.rs:367-388](apps/desktop/src-tauri/src/session.rs)
- A plan request is detected from `session/request_permission` when `toolCall.rawInput.plan` is a string, kind is `switch_mode`, or the title contains `ExitPlanMode` - [crates/core/src/acp.rs:350-359](crates/core/src/acp.rs) (function `is_plan_request`)
- Plan text comes from `toolCall.rawInput.plan`, with a fallback to tool call content text - [crates/core/src/acp.rs:361-376](crates/core/src/acp.rs) (function `plan_text`)
- On a plan request, Rust parses with `plan::parse` and `plan::check_paths` and sends `planText` and `plan` to the UI - [apps/desktop/src-tauri/src/session.rs:426-441](apps/desktop/src-tauri/src/session.rs)
- The format is enforced by `PLAN_INSTRUCTION` (Steps:, Files: `create|edit|delete <path> - <reason>`, Risk:), appended to the system prompt at `session/new` via `systemPrompt: {mode: "append"}` or `_meta.systemPrompt.append` for older adapters - [crates/core/src/agent.rs:9-27](crates/core/src/agent.rs), [crates/core/src/agent.rs:161-182](crates/core/src/agent.rs)
- `plan::parse(text)` is pure text in, `ParsedPlan { steps, files, risk, fallback }` out; fallback takes the first 6 top-level list items and paths found in the text - [crates/core/src/plan.rs:27-97](crates/core/src/plan.rs)
- ACP `plan` session updates (the agent's to-do list) only set `session.todo` for the live progress line, and are not used for the plan card - [apps/desktop/src/lib/stores/session.svelte.ts:338-340](apps/desktop/src/lib/stores/session.svelte.ts), [PLAN.md:143](PLAN.md)
- Live scope guard runs in the UI store: for edit/delete/move tool updates during a running plan, locations inside the root that are in `plan.files` tick as touched; others are added to `outside` in amber with a reason taken from the agent's recent text - [apps/desktop/src/lib/stores/session.svelte.ts:356-375](apps/desktop/src/lib/stores/session.svelte.ts)

Snapshots
- Shadow repo: `--git-dir=<appdata>/projects/<hash>/shadow.git --work-tree=<project>`; isolated config, no hooks, attributes `* -text -filter -ident -working-tree-encoding`, `--literal-pathspecs` on every call except `check-ignore` - [crates/core/src/snapshot.rs:1-6](crates/core/src/snapshot.rs), [crates/core/src/snapshot.rs:156-217](crates/core/src/snapshot.rs)
- `snapshot(refname)` scans for uncovered items (nested repos, over 20 MB, Windows symlinks), rewrites `info/exclude` from that list, `git add -A --ignore-errors .`, drops gitlinks, `write-tree`, `commit-tree` chained on `refs/fi/head`, and updates the ref - [crates/core/src/snapshot.rs:250-323](crates/core/src/snapshot.rs)
- `diff(a, b)` returns `Changes { files: [FileChange{path, old_path, status, added, removed, binary}], generated, added, removed }` from `--name-status -M` and `--numstat -M`, collapsing generated folders - [crates/core/src/snapshot.rs:79-107](crates/core/src/snapshot.rs), [crates/core/src/snapshot.rs:326-394](crates/core/src/snapshot.rs)
- Snapshot at project open (`refs/fi/opened`) runs in the background - [apps/desktop/src-tauri/src/commands.rs:133-140](apps/desktop/src-tauri/src/commands.rs)
- Retention: 50 turns, 14 days, 1 GB budget - [crates/core/src/snapshot.rs:17-24](crates/core/src/snapshot.rs)

Undo
- `preview` takes a `now` snapshot and lists conflicts (files changed after the turn ended, except your own viewer saves), commands run, not-covered and ignored-changed items - [apps/desktop/src-tauri/src/turns.rs:95-146](apps/desktop/src-tauri/src/turns.rs)
- `undo` snapshots `preundo`, restores turn files from `start` except kept conflicts and your edits, marks `undone`, and reports `exact` - [apps/desktop/src-tauri/src/turns.rs:158-205](apps/desktop/src-tauri/src/turns.rs)
- `redo` restores from the pre-undo snapshot; `revert_file` restores one file (and its rename source); `versions` gives before/after text for the diff viewer - [apps/desktop/src-tauri/src/turns.rs:207-288](apps/desktop/src-tauri/src/turns.rs)
- Undo is blocked while any turn runs in the project, and sets the session's `undo_note` so the next prompt tells the agent - [apps/desktop/src-tauri/src/commands.rs:334-371](apps/desktop/src-tauri/src/commands.rs)

Watcher
- `files::watch` uses `notify-debouncer-mini` (250 ms), emits `fs-changed { projectId, paths }` for every changed path except `.git`, and, while `project.watch_log` is Some (a running turn), appends the paths to it - [apps/desktop/src-tauri/src/files.rs:98-139](apps/desktop/src-tauri/src/files.rs)
- The watch log is only used at turn end to find ignored files that changed (files not in the snapshot diff, checked with `git check-ignore`) - [apps/desktop/src-tauri/src/session.rs:731-743](apps/desktop/src-tauri/src/session.rs)
- Viewer saves during a turn are recorded as "your edit" via `fs_write` - [apps/desktop/src-tauri/src/files.rs:61-79](apps/desktop/src-tauri/src/files.rs)

### Inferences
- Snapshots, diff, undo, redo, revert-file and `TurnRecord` are agent-agnostic. They only need two calls: "turn start" (claim + start snapshot + `watch_log = Some`) and "turn end" (end snapshot + diff + record + release). Those steps are inlined in `session.rs` (`prompt_inner`, `run_plan`, `finish_turn`), not factored out, so a second turn driver (pane mode or hooks) would either duplicate or first extract them into a shared helper on `Project`.
- The plan parser is source-agnostic (`plan::parse(&str)`); only the plan text acquisition (`acp::is_plan_request`, `acp::plan_text`) is ACP specific.
- The live scope guard depends on ACP `tool_call`/`tool_call_update` locations in the UI store. Without ACP, the closest existing signal is the watcher's `fs-changed` event, which has no reason text and cannot tell the agent's writes from other writes.

### Gaps
- No code reads files under the project (for example `.focuside/plan.md`) as a plan source; the only plan source is the ACP permission request.
- No ACP `plan` entries are ever compared with the parsed plan.

## Q3. Is there an existing terminal, and what does "terminal" mean in src-tauri

### Takeaway
There is a real terminal UI: a bottom "terminal drawer" (xterm.js over a portable-pty shell), hidden until opened with Ctrl+` or from the palette. It is separate from ACP; FocusIDE tells ACP agents it has no terminal capability. The owner's statement that "there is no terminal UI, only a chat box" is true of the default visible layout but not of the code.

### Cited Findings
- `pty.rs`: "The terminal drawer: one shell per project in a PTY. Output goes to the UI as `pty` events (base64 bytes); closing kills the shell's whole process tree." - [apps/desktop/src-tauri/src/pty.rs:1-2](apps/desktop/src-tauri/src/pty.rs)
- `pty_open(project_id, cols, rows, shell)` opens a portable-pty in the project root with `TERM=xterm-256color`, `COLORTERM=truecolor`; the program is the settings shell string (no args) or the default shell (pwsh, then powershell.exe, or `$SHELL -l`). A reader thread emits `pty` events in 16 KB chunks and `pty-exit` on EOF - [apps/desktop/src-tauri/src/pty.rs:38-124](apps/desktop/src-tauri/src/pty.rs)
- `pty_write`, `pty_resize`, `pty_close` (kills the `PidTree`) - [apps/desktop/src-tauri/src/pty.rs:126-156](apps/desktop/src-tauri/src/pty.rs)
- PTYs live in `AppState.ptys: Mutex<HashMap<String, Pty>>`, ids `pty1`, `pty2`... - [apps/desktop/src-tauri/src/lib.rs:15-24](apps/desktop/src-tauri/src/lib.rs), [apps/desktop/src-tauri/src/pty.rs:99-107](apps/desktop/src-tauri/src/pty.rs)
- `PidTree` attaches to a process someone else started (the PTY shell) so the tree can be killed - [crates/core/src/proc.rs:118-151](crates/core/src/proc.rs)
- `TerminalDrawer.svelte` uses `@xterm/xterm` 6 and `@xterm/addon-fit`, Cascadia Code, 5000 lines scrollback, a ResizeObserver, and can type a queued command (`app.terminalCommand`) into the shell 600 ms after start - [apps/desktop/src/lib/components/TerminalDrawer.svelte:1-121](apps/desktop/src/lib/components/TerminalDrawer.svelte), [apps/desktop/package.json](apps/desktop/package.json) (dependencies `@xterm/xterm ^6.0.0`, `@xterm/addon-fit ^0.11.0`)
- It is mounted under the conversation only when `app.terminalVisible` (default false), keyed by project - [apps/desktop/src/App.svelte:274-290](apps/desktop/src/App.svelte), [apps/desktop/src/lib/stores/app.svelte.ts:84-85](apps/desktop/src/lib/stores/app.svelte.ts)
- Keys: Ctrl+` toggles it; when the terminal has focus only Mod+` and Ctrl+Shift+key reach the app - [apps/desktop/src/App.svelte:198-213](apps/desktop/src/App.svelte), [PLAN.md:308](PLAN.md)
- `app.runInTerminal(command)` opens the drawer and runs install or sign-in commands - [apps/desktop/src/lib/stores/app.svelte.ts:543-547](apps/desktop/src/lib/stores/app.svelte.ts)
- README lists "Terminal: A terminal drawer that opens in the project folder." - [README.md:78](README.md)
- ACP "terminal" capability is off: `clientCapabilities.terminal = false` - [crates/core/src/acp.rs:287-298](crates/core/src/acp.rs)
- Shell commands the agent runs are seen only as ACP `execute` tool calls (title is the command) - [apps/desktop/src-tauri/src/session.rs:352-364](apps/desktop/src-tauri/src/session.rs)
- PLAN.md risk table: "Claude in the terminal drawer stays available" as the subscription fallback - [PLAN.md:399](PLAN.md)

### Inferences
- A pane mode can reuse almost all of the PTY plumbing. The concrete gaps are: `pty_open` only takes a shell string with no args or extra env (`pty.rs:76-79,88-92`); it is per project, not per session; the drawer is a bottom panel, not the center pane; and the `pty` event stream is not tied to a session key.

### Gaps
- The terminal drawer's behavior with full-screen TUIs (alternate screen, mouse, bracketed paste) such as Claude Code's own UI has not been tested per PLAN.md; only "pwsh in the project folder" was checked (PLAN.md:490).

## Q4. What would need to change for (a) pane mode, (b) a Claude Code plugin with hooks, (c) a per-agent or per-auth mode selector, (d) a model-free diff summary

### Takeaway
(a) and (c) mostly extend existing seams (`AgentSpec`, `session_start`, `pty.rs`, `TerminalDrawer.svelte`); the large work is a second turn driver that does not depend on ACP. (b) needs an IPC path from a hook process to the running app, which does not exist today beyond the single-instance argv forward, and it conflicts with "never write into the user's repo" if files go in `.focuside/` inside the project. (d) is close: `Changes`, `TurnRecord` and `turns::versions` already hold everything a deterministic summary needs.

### Cited Findings and landing points

(a) Pane mode: the agent's own TUI in a PTY with xterm.js
- Reuse: `portable-pty` and xterm are already dependencies - [apps/desktop/src-tauri/Cargo.toml](apps/desktop/src-tauri/Cargo.toml) (`portable-pty = "0.9"`), [apps/desktop/package.json](apps/desktop/package.json)
- Landing point, backend: `pty_open` would need a program plus args plus env (for example a session id or IPC address for hooks) and should key by session. Today: `shell: Option<String>` only, args only for the default shell, env only TERM and COLORTERM - [apps/desktop/src-tauri/src/pty.rs:60-93](apps/desktop/src-tauri/src/pty.rs)
- Landing point, launch command: `AgentSpec` has only ACP `program`/`args`; a TUI command per agent (for example plain `claude`, `codex`, `gemini`) would be a new field or a new table - [crates/core/src/agent.rs:186-239](crates/core/src/agent.rs)
- Landing point, session model: `Session` (Rust) is built around an ACP `Connection`, `AgentCommand`, `Capabilities`, permission map and `TurnCtx` - [apps/desktop/src-tauri/src/session.rs:26-69](apps/desktop/src-tauri/src/session.rs). The UI `Session` holds `entries` (conversation), `waiting`, `todo`, `configOptions` - [apps/desktop/src/lib/stores/session.svelte.ts:69-98](apps/desktop/src/lib/stores/session.svelte.ts). A pane session has none of the ACP stream, so it needs either a new session kind or a variant flag.
- Landing point, center view: `App.svelte` renders `ClaudeAccess`/`EnvBanner`, `Conversation`, `MessageBox` for a session; a pane session would render an xterm host there instead - [apps/desktop/src/App.svelte:261-273](apps/desktop/src/App.svelte)
- Turn boundaries: in ACP mode, a turn is the `session/prompt` request (session.rs:686-695). In pane mode the user types into the TUI, so FocusIDE cannot see prompt start or end from the PTY byte stream; it needs hooks (see (b)) or a heuristic.
- Stop and Pause: today `session/cancel` then kill after 3 s - [apps/desktop/src-tauri/src/session.rs:844-877](apps/desktop/src-tauri/src/session.rs). Pane mode would write an interrupt key to the PTY or kill the `PidTree`.
- Plan first, Run plan, Discard, model picker, slash command list, context usage, session list and resume are all ACP driven (`set_mode`, `run_plan_option`, `config_option_update`, `available_commands_update`, `usage_update`, `session/list`) - [apps/desktop/src-tauri/src/session.rs:540-571](apps/desktop/src-tauri/src/session.rs), [apps/desktop/src-tauri/src/session.rs:904-915](apps/desktop/src-tauri/src/session.rs), [apps/desktop/src/lib/stores/session.svelte.ts:338-353](apps/desktop/src/lib/stores/session.svelte.ts). In pane mode the TUI provides them itself.
- The undo note ("The user reverted all file changes from your last turn.") is injected as the first block of the next ACP prompt - [apps/desktop/src-tauri/src/session.rs:660-669](apps/desktop/src-tauri/src/session.rs). In pane mode there is no prompt to prepend to.
- The plan format instruction is injected through ACP `systemPrompt` - [crates/core/src/agent.rs:161-175](crates/core/src/agent.rs). In pane mode it would need another channel.
- Keyboard: the existing "terminal keeps its keys" rule would apply to the pane - [apps/desktop/src/App.svelte:198-213](apps/desktop/src/App.svelte)
- Process cleanup: PTY children are attached with `PidTree::attach` and killed on close or drop - [apps/desktop/src-tauri/src/pty.rs:21-34](apps/desktop/src-tauri/src/pty.rs)
- Auth: the `claude_access` gate is in the ACP start path only (`session.rs:136-148`), so a pane session that runs plain `claude` is not gated today.

(b) Claude Code plugin with hooks that notify FocusIDE and write `.focuside/plan.md` and `summary.md`
- No local IPC server exists. The only inbound channel from another process is `tauri_plugin_single_instance`, which forwards a second launch's argv to the running app and emits `open-paths` - [apps/desktop/src-tauri/src/lib.rs:58-66](apps/desktop/src-tauri/src/lib.rs). `folder_args` drops every argument that starts with `-` or is not an existing folder - [apps/desktop/src-tauri/src/lib.rs:26-42](apps/desktop/src-tauri/src/lib.rs)
- The watcher already sees any file written inside the project, including a `.focuside/` folder, and emits `fs-changed` - [apps/desktop/src-tauri/src/files.rs:98-128](apps/desktop/src-tauri/src/files.rs). A file-drop protocol could ride on this without a new server.
- Snapshot exclusion: `info/exclude` is rewritten on every snapshot from the not-covered list only - [crates/core/src/snapshot.rs:255-270](crates/core/src/snapshot.rs). A `.focuside/` folder written by hooks would show up in the changes card and be reverted by Undo turn unless `/.focuside/` is added there (and to the watcher's skip list at `files.rs:114`).
- Turn start and end calls a hook would trigger map onto: claim + start snapshot + watch log (session.rs:619-658, 486-502) and end snapshot + diff + record + release (session.rs:723-828).
- Tool-level data that ACP gives today and hooks would have to supply for parity: edit paths (`TurnCtx.edits`, used for the "outside" flag), shell commands (`TurnCtx.commands`, listed as "Not undone" in the undo dialog) - [apps/desktop/src-tauri/src/session.rs:32-42](apps/desktop/src-tauri/src/session.rs), [apps/desktop/src-tauri/src/turns.rs:30-35](apps/desktop/src-tauri/src/turns.rs)
- Plan text could be fed to the existing parser unchanged: `plan::parse(&text)` then `plan::check_paths` - [crates/core/src/plan.rs:37-112](crates/core/src/plan.rs)
- Tauri capabilities: the window has only core, dialog, opener, clipboard, notification, updater and process permissions; any new command must be added to `generate_handler!` - [apps/desktop/src-tauri/capabilities/default.json](apps/desktop/src-tauri/capabilities/default.json), [apps/desktop/src-tauri/src/lib.rs:93-129](apps/desktop/src-tauri/src/lib.rs)
- The `focuside` CLI shim on PATH is not built yet (packaging, v2/v3) - [PLAN.md:506-507](PLAN.md)

(c) Per-agent and per-auth mode selector
- Current selector: Settings "Default agent" select plus palette "New session with <agent>" - [apps/desktop/src/lib/components/Settings.svelte:77-103](apps/desktop/src/lib/components/Settings.svelte), [PLAN.md:514](PLAN.md)
- Persisted settings shape: `{ theme, fontSize, shell, agent, agentCommand, autoUpdate, showIgnored, notifications }` - [apps/desktop/src/lib/stores/app.svelte.ts:14-25](apps/desktop/src/lib/stores/app.svelte.ts). A mode would be a new field here, or derived per session.
- Saved sessions store `agent` only; resume reuses it - [apps/desktop/src/lib/stores/session.svelte.ts:59-67](apps/desktop/src/lib/stores/session.svelte.ts). A pane session cannot be resumed through ACP `session/resume`, so `SavedSession` would need a mode field too.
- Backend entry: `session_start(project_id, resume, agent_id, custom_command)` - [apps/desktop/src-tauri/src/commands.rs:163-202](apps/desktop/src-tauri/src/commands.rs). A `mode` argument would land here.
- Auth input for an automatic choice: `agent::claude_auth()` and `ClaudeAccess` - [crates/core/src/agent.rs:87-150](crates/core/src/agent.rs)
- Settings already has 7 always-visible rows (theme, font size, shell, default agent, show ignored, notifications, automatic updates) plus a conditional custom command row - [apps/desktop/src/lib/components/Settings.svelte:53-129](apps/desktop/src/lib/components/Settings.svelte)

(d) Model-free diff summary from snapshots
- Data already available at turn end: `Changes` with per-file status (create, edit, delete, rename with `old_path`), `added`/`removed` line counts, `binary`, generated-folder counts and totals - [crates/core/src/snapshot.rs:70-107](crates/core/src/snapshot.rs)
- Plus `TurnRecord` `commands`, `edits`, `your_edits`, `ignored_changed`, `not_covered` - [apps/desktop/src-tauri/src/turns.rs:19-45](apps/desktop/src-tauri/src/turns.rs)
- Plus the plan's file list for "Not in plan" marks (`Session.notInPlan`) - [apps/desktop/src/lib/stores/session.svelte.ts:474-480](apps/desktop/src/lib/stores/session.svelte.ts)
- Full before/after text per file via `turns::versions` (and `ShadowRepo::read_file`) - [apps/desktop/src-tauri/src/turns.rs:252-288](apps/desktop/src-tauri/src/turns.rs), [crates/core/src/snapshot.rs:437-442](crates/core/src/snapshot.rs)
- Natural place to compute it: `finish_turn` right after `repo.diff` (session.rs:730), or in core next to `collapse_generated` - [apps/desktop/src-tauri/src/session.rs:723-813](apps/desktop/src-tauri/src/session.rs), [crates/core/src/snapshot.rs:747-748](crates/core/src/snapshot.rs)
- The changes card today shows a header with totals, one row per file with +/-, Not-in-plan notes and generated lines - [apps/desktop/src/lib/components/ChangesCard.svelte:69-102](apps/desktop/src/lib/components/ChangesCard.svelte)
- `similar` is listed in PLAN.md's packages but is not a dependency of core or desktop; there is no Rust hunk-level diff today (the UI diff viewer uses `@codemirror/merge`) - [PLAN.md:198](PLAN.md), [crates/core/Cargo.toml](crates/core/Cargo.toml), [apps/desktop/src-tauri/Cargo.toml](apps/desktop/src-tauri/Cargo.toml), [apps/desktop/package.json](apps/desktop/package.json)

### Inferences
- Main refactor for (a) and (b): extract a `TurnRecorder` (claim, start snapshot, watch log, end snapshot, diff, record, release, prune) out of `session.rs` so both the ACP `Session` and a new pane or hook driver call it. Without that, pane mode would copy around 150 lines of `finish_turn`.
- For (b) the cheapest IPC that fits existing code is file drops watched by the existing watcher, but it must write outside the project (for example the project's app data folder, passed to the PTY child as an env var by `pty_open`) to keep the "nothing written to the user's repo" rule. Writing `.focuside/plan.md` in the project conflicts with that rule and needs snapshot and watcher exclusions. Re-launching `FocusIDE.exe --hook ...` through single-instance is possible but would start a GUI exe per hook call; `folder_args` also filters flags out today.
- The "outside this session" heuristic (session.rs:757-762) would mark every pane-mode change as outside when `edits` is empty and no commands were recorded; it needs a mode guard.
- (c) An automatic default fits PLAN's "If a choice has a sensible default, do not ask": Claude with `Allowed` access gives ACP mode, `Subscription` gives pane mode, other agents default to ACP when they speak it. This avoids an extra Settings row.
- (d) A deterministic summary (for example "4 files: 2 edited, 1 created, 1 renamed; +38 -6; 214 generated files in target/; commands run: npm install; 1 file not in plan") needs no new data. Hunk-level text (function names touched) would need either `similar` in Rust or reuse of git `diff -U0` from the shadow repo.

### Gaps
- Exact Claude Code hook event names, payloads, plugin packaging, and CLI flags for appending a system prompt in the TUI are outside this repo; they must come from Claude Code docs (other researchers).
- The repo does not show how a TUI handles ExitPlanMode approval in a way FocusIDE could intercept; in pane mode the user approves inside the TUI, so the "Run plan" start snapshot point (session.rs:486-494) has no equivalent without a hook.

## Q5. Existing decisions in PLAN.md that conflict with these ideas

### Takeaway
Several written decisions push against pane mode and in-repo plugin files: "ACP only", "No stream-json fallback", "Nothing is written to the user's repo", "Settings max 6 rows", "The agent's text is shown as a document, not as a chat", and "the center of the screen is the conversation and its cards". Others support the ideas: the policy note that unmodified `claude` with a subscription is allowed, "Claude in the terminal drawer stays available", "No second model call", and "Snapshots, undo and the changes card do not depend on the agent".

### Cited Findings

Decisions that conflict or need an explicit change
- "Agents: ACP only" and "No stream-json fallback. Known plan approval bugs over stdio make it a trap." - [PLAN.md:128-132](PLAN.md); decision row "ACP only, Claude Code only in v1 | Same path as Zed. stream-json has known plan approval bugs" - [PLAN.md:443](PLAN.md)
- Plan instruction must not write into the user's repo: "Find how to add the plan format instruction (adapter system prompt option or another way that does not write into the user's repo)" - [PLAN.md:345](PLAN.md); "Nothing is written to the user's repo" - [PLAN.md:466](PLAN.md); code comment "Nothing is written into the user's repo" - [crates/core/src/agent.rs:9-11](crates/core/src/agent.rs). Writing `.focuside/plan.md` and `summary.md` inside the project conflicts with this.
- Shadow snapshots "never touch the user's repo" - [PLAN.md:151](PLAN.md), [PLAN.md:448](PLAN.md)
- Settings has at most 6 rows - [PLAN.md:65](PLAN.md), [PLAN.md:451](PLAN.md). The code already has 7 rows (see Q4 (c)), so a mode row would widen an existing breach.
- "Core features (plan card, changes card, undo, viewer, editing) have no off switch" - [PLAN.md:63](PLAN.md). In pane mode the plan card cannot work the same way (no ExitPlanMode interception), which reads as a mode that switches a core feature off.
- "Insight. ... the center of the screen is the conversation and its cards, not an editor." and "The agent's text is shown as a document, not as a chat." - [PLAN.md:28](PLAN.md), [PLAN.md:74](PLAN.md). A TUI in the center replaces the document view.
- Scope guard is defined on "ACP tool-call locations" - [PLAN.md:146](PLAN.md)
- "Permission requests show the adapter's own options" and "Run plan answers the plan request with the adapter's accept edits option" - [PLAN.md:133-134](PLAN.md). In pane mode the TUI asks itself.
- "FocusIDE never reads, stores or proxies credentials. It only starts the user's installed agent." - [PLAN.md:135](PLAN.md), [AGENTS.md](AGENTS.md) (Rules, Credentials). A plugin or hook must not read tokens; reading only `claude auth status` method and provider is the established limit (agent.rs:87-88).
- Owner decision to block Claude plan logins in FocusIDE - [PLAN.md:533](PLAN.md). Pane mode for subscription users reverses the user-facing outcome of that decision, so it needs a new owner decision row.
- Finding Claude Code sessions started in a normal terminal is planned for v2, not v1 - [PLAN.md:181](PLAN.md); "outside sessions" and "terminal tabs" are listed for v2 - [PLAN.md:325](PLAN.md)
- Release copy (README box, llms.txt, en.ts access strings) all say Claude plans cannot be used - [README.md:100-106](README.md), [site/llms.txt:5](site/llms.txt), [apps/desktop/src/lib/i18n/en.ts:202-216](apps/desktop/src/lib/i18n/en.ts)

Decisions that support the ideas
- Policy check: "Allowed, in plain words: an end user signs in to the unmodified Claude Code binary with their own subscription"; way (c) proposes driving the unmodified `claude` CLI, "closer to ordinary use of Claude Code" - [PLAN.md:525](PLAN.md), [PLAN.md:529](PLAN.md)
- Risk mitigation "Claude in the terminal drawer stays available" - [PLAN.md:399](PLAN.md); first-run "Not signed in: Sign in to Claude Code opens the terminal drawer with `claude` running" - [PLAN.md:287](PLAN.md)
- "Short plan from a fixed format and a parser, no second model call" (supports a model-free summary) - [PLAN.md:446](PLAN.md)
- "Real measurements over claims: the changes card reads the disk, not the agent's words" (supports a diff-based summary) - [PLAN.md:86](PLAN.md)
- "Snapshots, undo and the changes card do not depend on the agent" - [PLAN.md:514](PLAN.md); README "Snapshots, the scope guard and Undo turn work with every agent" - [README.md:121](README.md)
- "Features appear in context ... an agent only when it is installed" and "If a choice has a sensible default, do not ask" (supports an automatic mode choice from detected auth) - [PLAN.md:64-66](PLAN.md)
- PLAN already lists xterm.js and portable-pty, and Linux xterm renderer notes - [PLAN.md:188](PLAN.md), [PLAN.md:198-199](PLAN.md)
- "Up next: M4, two weeks of daily use" and "v2 is planned only from this list" (docs/daily-use.md is still empty) - [PLAN.md:7](PLAN.md), [docs/daily-use.md](docs/daily-use.md)

### Inferences
- Pane mode plus hooks is close to PLAN way (c) in spirit (unmodified `claude`), but uses the interactive TUI instead of headless stream-json, so it avoids the "stream-json plan approval bugs" reason while still breaking "ACP only". A new decision row in PLAN.md section 10 would be needed, and the owner's Release setup decision (PLAN.md:533) would need to be revised.
- If hook output must go into the project, the least conflicting approach is a folder that the snapshot excludes and that the user's `.gitignore` is not required to know about. The current code has no such exclusion.

### Gaps
- PLAN.md does not record why "stream-json has known plan approval bugs" in detail, or whether that also affects hook-based plan capture in the interactive TUI.
- Whether Anthropic's policy treats a GUI that hosts the interactive `claude` TUI in a PTY (plus hooks) as "ordinary use of the unmodified binary" is not settled in the repo (PLAN.md:529 says way (c) is "still not confirmed by Anthropic").
