<p align="center">
  <img src="apps/desktop/app-icon.svg" width="96" height="96" alt="FocusIDE logo">
</p>

<h1 align="center">FocusIDE</h1>

<p align="center">
  <b>The simplest IDE for working with AI coding agents.</b><br>
  You read, approve and check. The agent types.
</p>

<p align="center">
  <a href="https://github.com/yobotn/focuside/releases/latest"><img src="https://img.shields.io/github/v/release/yobotn/focuside?label=release&color=22c55e" alt="Latest release"></a>
  <a href="https://github.com/yobotn/focuside/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/yobotn/focuside/ci.yml?branch=main&label=CI" alt="CI status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License: Apache 2.0"></a>
  <img src="https://img.shields.io/badge/platform-Windows-0078d4" alt="Platform: Windows">
</p>

<p align="center">
  <a href="https://github.com/yobotn/focuside/releases/latest"><b>Download for Windows</b></a>
  &nbsp;·&nbsp;
  <a href="https://yobotn.github.io/focuside/">Website</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/yobotn/focuside/issues/new/choose">Report a bug</a>
</p>

<p align="center">
  <img src="docs/screenshot.png" alt="FocusIDE showing a finished plan. One file the agent changed outside the plan is marked Not in plan, with Review and Undo turn buttons." width="900">
</p>

FocusIDE is a free, open source desktop app for people who use coding agents every day. It starts the agent you already have installed, such as Claude Code, Codex or Gemini CLI. It shows you a short plan before any file changes, watches what the agent touches, and lets you undo a whole turn with one key.

It is not an editor with a chat box bolted on. The conversation is the center of the window. Files, diffs and the terminal are there when you need to check the work.

## Why FocusIDE

- **Plan first.** The agent shows a plan card before it edits anything: at most 6 steps, every file it will create, edit or delete, and one line of risk. Run it with `Ctrl+Enter`, change it, or throw it away.
- **Scope guard.** While the agent works, any file it touches that was not in the plan shows up live in amber, with a Pause button. You see scope creep when it happens, not after.
- **Undo turn.** Every turn starts with a snapshot. One key puts the project back exactly as it was, byte for byte. It works in any folder, git repo or not, and never touches your own git history.
- **One window, every project.** A rail on the left holds your projects. The waiting queue (`Ctrl+J`) jumps to the next session that needs an answer from you, across all projects.
- **Any ACP agent.** Claude Code works out of the box with an API key. Codex, Gemini CLI, OpenCode, Qwen Code, Goose or any custom command that speaks the Agent Client Protocol work too.

## Features

- Plan card with steps, files and risk. Files tick off as the agent edits them.
- Changes card after each turn, with line counts, a diff viewer and Undo turn.
- Revert a single file from a turn. Redo the last undo.
- Permission prompts inline in the conversation. Edits inside the plan run without asking.
- Stop (`Ctrl+.`) cancels the turn and ends every process the agent started.
- Sessions: several per project, named, saved and resumed with their history.
- File tree with clear icons, a fast fuzzy file picker and project-wide search.
- Viewer for code and diffs. Markdown files render as documents, with tables, task lists, alerts and images.
- Terminal drawer in the project folder.
- Command palette, model picker, `@` to mention files, `/` for agent commands.
- Resizable panels. Move the file tree and viewer to either side.
- Light and dark themes. Keyboard first, with full mouse support.

## Download

Get the Windows installer from the [latest release](https://github.com/yobotn/focuside/releases/latest). The app updates itself when a new version is out.

macOS and Linux build in CI and ship later. For now, build them from source.

### Requirements

- **An agent.** FocusIDE runs the agent you install and set up yourself. For Claude Code: `npm install -g @anthropic-ai/claude-code`, plus an API key or a cloud provider (see below). Other agents are listed in Settings.

**Claude needs an API key.** Anthropic does not allow apps built on the Claude Agent SDK to use Claude plans (Pro, Max, Team, Enterprise). FocusIDE connects to Claude Code through an Agent SDK based adapter, so it starts Claude only with an API key (`ANTHROPIC_API_KEY`, from the [Claude Console](https://platform.claude.com/settings/keys)) or a cloud provider (Amazon Bedrock, Google Vertex AI, Microsoft Foundry). Claude Code reads the key; FocusIDE never reads or stores it. See [Anthropic's rules](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use).
- **Git.** Undo turn uses a private git repo for snapshots. Your project does not need to be a git repo.

## Keyboard shortcuts

`Ctrl` is `Cmd` on macOS.

| Keys | Action |
| --- | --- |
| `Ctrl+K` | Command palette |
| `Ctrl+P` | Open a file |
| `Ctrl+Shift+F` | Search in the project |
| `Ctrl+Enter` | Run the waiting plan |
| `Ctrl+.` | Stop the running turn |
| `Ctrl+J` | Waiting queue, across projects |
| `Ctrl+E` | Sessions in this project |
| `Ctrl+B` | Show or hide the file tree |
| `` Ctrl+` `` | Show or hide the terminal |
| `Ctrl+S` | Save the file in the viewer |
| `R` / `U` | Review or undo a finished turn (on the changes card) |
| `J` / `K` | Next or previous file in the diff viewer |
| `Esc` | Close an overlay or the viewer. Stops a running turn when nothing is open |

## How it works

1. **FocusIDE starts your agent** over the [Agent Client Protocol](https://agentclientprotocol.com) (ACP). Claude Code connects through the bundled `claude-agent-acp` adapter, which runs your own installed `claude` binary.
2. **Before each turn, it takes a snapshot** of the project in a separate git repo inside the app's data folder. Your repo, branches and history are never touched.
3. **During the turn, it compares** each file the agent touches with the plan. After the turn, it shows the changes and keeps the snapshot so you can undo.

**Your code stays on your machine.** FocusIDE has no server, no account and no telemetry. It never reads, stores or proxies your agent credentials. The agent talks to its own provider exactly as it does in your terminal.

## Build from source

Needs Rust (stable), Node 22+ and git.

```sh
npm run setup                    # UI dependencies
node scripts/fetch-adapter.mjs   # pinned claude-agent-acp binary for this OS
npm run dev                      # run the app in development
npm run lint                     # clippy, rustfmt, text lint, strings, svelte-check
npm test                         # Rust tests
npm run build                    # installer for this OS
```

Open a folder from a terminal with `FocusIDE.exe <folder>`. A running window takes the folder.

The stack is Tauri 2 and Rust for the app shell, agent connection, snapshots and terminal, with Svelte 5, Tailwind 4 and CodeMirror 6 for the UI. See [PLAN.md](PLAN.md) for the product plan and the build results, and [AGENTS.md](AGENTS.md) if you use a coding agent on this repo.

## Report a bug

Open an [issue on GitHub](https://github.com/yobotn/focuside/issues/new/choose). You can also use **Report an issue** in the app, which fills in your version and OS. Security problems go to [private security advisories](https://github.com/yobotn/focuside/security/advisories/new), not public issues.

## Contributing

Pull requests are welcome. Please open an issue first for anything larger than a small fix, so we can agree on the approach. Run `npm run lint` and `npm test` before you push. UI text is short and literal, with no emojis and no em dashes (the text lint checks this).

## License

[Apache 2.0](LICENSE).

## Trademarks

FocusIDE is an independent project, not affiliated with or endorsed by Anthropic. Claude and Claude Code are trademarks of Anthropic. Other agent names are trademarks of their owners. FocusIDE works with these agents. It does not include them.
