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
npm run dev                      # the app
npm run lint                     # clippy, fmt, text lint, strings, svelte-check
npm test                         # Rust tests
```

See [PLAN.md](PLAN.md) for the product plan.

## License

Apache 2.0.
