# AGENTS.md

Instructions for coding agents working on this repo. Humans: see [README.md](README.md) and [PLAN.md](PLAN.md).

## What this is

FocusIDE is a Tauri 2 desktop app. It starts an ACP coding agent (Claude Code through the bundled `claude-agent-acp` adapter, or another ACP agent) and shows plans, changes and undo around it.

## Layout

| Path | What |
| --- | --- |
| `crates/core` | Rust library: ACP client (`acp.rs`), agent lookup (`agent.rs`), plan parser (`plan.rs`), process trees (`proc.rs`), shadow git snapshots (`snapshot.rs`), files and search (`files.rs`) |
| `crates/spike` | Command line tool used to test ACP behavior without the UI |
| `apps/desktop/src-tauri` | Tauri app: commands, sessions, turns, watcher, terminal |
| `apps/desktop/src` | Svelte 5 UI. Stores in `lib/stores`, components in `lib/components`, strings in `lib/i18n/en.ts` |
| `scripts` | Lint, i18n check, adapter download, icon build |
| `site` | Static website for focuside.dev (GitHub Pages) |
| `PLAN.md` | Product plan, decisions and build results. Read section 10 before larger changes |

## Commands

Run from the repo root.

```sh
npm run setup                    # install UI dependencies (once)
node scripts/fetch-adapter.mjs   # download the pinned adapter binary (once per OS)
npm run dev                      # run the app
npm run lint                     # clippy -D warnings, rustfmt --check, text lint, i18n check, svelte-check
npm test                         # cargo test --workspace
npm run build                    # release installer
```

`npm run lint` and `npm test` must pass before you commit.

## Rules

- **Text style.** UI strings and docs use short, literal sentences. No idioms. No emojis (the root README is the only exception). No em dashes. `node scripts/text-lint.mjs` checks this.
- **Strings.** Every UI string lives in `apps/desktop/src/lib/i18n/en.ts`. `scripts/i18n-check.mjs` fails on unused or missing keys.
- **Credentials.** FocusIDE never reads, stores or proxies agent credentials. Do not add code that does.
- **Git.** Commit locally. Do not push or create remote repos unless the owner asks.
- **System tools.** Do not install system tools without approval. Project npm and cargo packages are fine.
- **Async.** In the desktop crate, spawn with `tauri::async_runtime::spawn`, not `tokio::spawn` (sync commands run outside the Tokio runtime).
- **Snapshots.** Every git call on the shadow repo uses `--literal-pathspecs`, except `check-ignore`, which rejects it.
- **Windows first.** Test on Windows. macOS and Linux build in CI.
