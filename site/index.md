# FocusIDE: the simplest IDE for working with AI agents

You read, approve and check. The agent types.

FocusIDE is a free, open source desktop app for AI coding agents. It shows a short plan before any file changes, marks scope creep while it happens, and undoes a whole turn with one key. It works with Claude Code, Codex, Gemini CLI and any ACP agent. No account. No telemetry.

- [Download for Windows](https://github.com/yobotn/focuside/releases/latest)
- [View on GitHub](https://github.com/yobotn/focuside)

## Key facts

- **What:** a desktop app for AI coding agents
- **Price:** free, Apache 2.0
- **Runs:** on your machine, no server
- **Agents:** Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Goose, any ACP agent
- **Platforms:** Windows 10 and 11 installer. macOS and Linux from source.

## How it works

1. **Ask.** Type what you want. With Plan first on, the agent answers with a plan card: at most 6 steps, every file it will create, edit or delete, and one line of risk.
2. **Run.** Press Ctrl+Enter. Edits inside the plan go through without asking. Files tick off as the agent works. A file outside the plan shows up in amber, with Pause.
3. **Keep or undo.** The changes card lists every file with line counts. Review the diff, revert one file, or press U to put the whole project back, byte for byte.

## Features

- **Plan first.** See the steps and the file list before anything changes. Run it, change it, or throw it away.
- **Scope guard.** Every file the agent touches is checked against the plan, live.
- **Undo turn.** A snapshot before every turn. One key restores the project exactly. Works in any folder and never touches your git history.
- **Waiting queue.** Ctrl+J jumps to the next session that needs an answer from you, across projects.
- **Check the work.** Diff viewer, file tree, fuzzy file picker, project search, terminal drawer. Markdown files render as documents.
- **Stays out of the way.** One window. Resizable panels on either side. Light and dark. Keyboard first.

## Bring your own agent

FocusIDE starts the agent you already use, over the open [Agent Client Protocol](https://agentclientprotocol.com). You install and set up the agent yourself. Usage is billed by its provider, to your own account.

**Claude needs an API key.** Anthropic does not allow apps built on the Claude Agent SDK to use Claude plans (Pro, Max, Team, Enterprise). FocusIDE connects to Claude Code through an Agent SDK based adapter, so it starts Claude only with an API key (`ANTHROPIC_API_KEY`, from the [Claude Console](https://platform.claude.com/settings/keys)) or a cloud provider (Amazon Bedrock, Google Vertex AI, Microsoft Foundry). Claude Code reads the key; FocusIDE never reads or stores it. See [Anthropic's rules](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use).

## Your code stays on your machine

- No server, no account, no telemetry.
- FocusIDE never reads, stores or proxies agent credentials.
- Snapshots live in a private repo in the app's data folder.
- Open source, so you can check all of this.

## Questions

**What is FocusIDE?**
A free, open source desktop app for working with AI coding agents. The conversation with the agent is the center of the window. It shows a plan before any file changes, marks files the agent changes outside the plan, and can undo a whole turn with one key.

**Is FocusIDE free?**
Yes. Apache 2.0. You pay only for your agent, under your own plan with its provider.

**Which AI agents does FocusIDE work with?**
Claude Code works with an API key or a cloud provider. Codex, Gemini CLI, OpenCode, Qwen Code and Goose work when installed. Any other ACP agent works through a custom command.

**Does FocusIDE see my code or my credentials?**
No. No server, no account, no telemetry. It never reads, stores or proxies agent credentials.

**Does my project need to be a git repo?**
No. Undo turn keeps snapshots in a separate private git repo inside the app's data folder.

**How does Undo turn work?**
Before each turn, FocusIDE takes a snapshot. Undo turn restores every file the agent changed to its exact bytes and removes files the agent created. Files you saved yourself during the turn are kept by default.

**Which operating systems does FocusIDE support?**
Windows 10 and 11 have an installer that updates itself. macOS and Linux build from source today.

**Is FocusIDE made by Anthropic?**
No. FocusIDE is an independent project, not affiliated with or endorsed by Anthropic. Claude and Claude Code are trademarks of Anthropic.

## Get started

1. Install your agent. For Claude Code: `npm install -g @anthropic-ai/claude-code`, then set `ANTHROPIC_API_KEY` from the Claude Console (or use a cloud provider).
2. [Download FocusIDE](https://github.com/yobotn/focuside/releases/latest) and run the installer.
3. Open a folder, ask for a change, and press Ctrl+Enter on the plan.
