# FocusIDE

The simplest IDE for working with AI agents. You read, approve and check. The agent types.

- **Plan card.** At most 6 steps plus the files the agent will touch. Run it with one key.
- **Scope guard.** While the agent runs, files it touches that were not in the plan show up live in amber, with Pause.
- **Undo turn.** One key restores the project to how it was before the turn, git repo or not.
- **Waiting queue.** Jump to the next session that needs you, across all projects.

Windows first. macOS and Linux are built in CI and ship later. Claude Code is the first agent, connected through ACP.

## Develop

Needs Rust, Node 22+, git and Claude Code (`claude`) on PATH.

```
npm run setup                    # UI dependencies
node scripts/fetch-adapter.mjs   # pinned claude-agent-acp binary for this OS
npm run dev                      # the app (builds the file icons first)
npm run lint                     # clippy, fmt, text lint, strings, svelte-check
npm test                         # Rust tests
```

Open a folder from a terminal with `FocusIDE.exe <folder>` (the `focuside` shim comes with the installer later). A running window takes the folder.

Dev builds print UI errors and session events to stderr.

See [PLAN.md](PLAN.md) for the product plan and build results, and [docs/daily-use.md](docs/daily-use.md) for the M4 notes.

## License

Apache 2.0.
