# GitHub Copilot CLI (plus Cursor CLI and Kiro CLI) terms and technical facts for a third-party ACP desktop client

Research date: 2026-11-12. Sources are web pages fetched on that date. Some quotes are short excerpts returned by a summarizing fetch tool, so check them against the live page before publishing. Dates are given for every changelog item.

## 1. May a third-party GUI drive Copilot CLI programmatically with a user's Copilot subscription?

### Takeaway
Yes, as far as the public record shows. GitHub ships an official ACP server in Copilot CLI (public preview since 2026-01-28) and names "custom frontends" and "IDE integrations" as intended use cases. It also ships a GA Copilot SDK (2026-06-02) for embedding the agent in "your own applications", and it officially endorsed a third-party open source client (OpenCode, 2026-01-16). No GitHub terms page found forbids a third-party client from spawning the official `copilot` binary. The restrictions that remain are general ones: the Acceptable Use Policies, a one-person login, no sharing tokens to exceed rate limits, and the Copilot CLI license terms if FocusIDE bundles the binary.

### Cited Findings

Official programmatic channels
- ACP changelog, 2026-01-28: "GitHub Copilot CLI now implements the Agent Client Protocol (ACP)." Third-party tools, IDEs and automation systems can connect. Clients can initialize, create sessions with custom working directories, send prompts with text, images and context, receive streaming updates, answer tool permission requests and cancel. Listed use cases: IDE integrations, CI/CD pipelines, custom frontends, multi-agent systems. [GitHub changelog](https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/)
- ACP server reference: "ACP support in GitHub Copilot CLI is in public preview and subject to change." Use cases include "custom frontends". [GitHub Docs, ACP server](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)
- Copilot SDK GA, 2026-06-02: "The GitHub Copilot SDK is now generally available." It lets you "embed GitHub Copilot's agentic engine into your own applications, services, and developer tools." Auth: "GitHub OAuth, GitHub Apps, environment tokens, and BYOK for OpenAI, Microsoft Foundry, Anthropic, and other providers." It "is available to all existing GitHub Copilot subscribers, including Copilot Free for personal use", and "to non-Copilot users via BYOK." It supports multi-client sessions. [GitHub changelog](https://github.blog/changelog/2026-06-02-copilot-sdk-is-now-generally-available/)
- Earlier SDK milestones: technical preview 2026-01-14, public preview 2026-04-02. [Changelog, technical preview](https://github.blog/changelog/2026-01-14-copilot-sdk-in-technical-preview/); [Changelog, public preview](https://github.blog/changelog/2026-04-02-copilot-sdk-in-public-preview/)

Official endorsement of a third-party client
- OpenCode changelog, 2026-01-16: "GitHub is officially supporting using your Copilot Pro, Pro+, Business, or Enterprise subscription with OpenCode." "GitHub Copilot now fully supports authentication with OpenCode through a formal partnership." Auth is the GitHub device login flow started inside OpenCode. [GitHub changelog](https://github.blog/changelog/2026-01-16-github-copilot-now-supports-opencode/)
- Note: that endorsement covers OpenCode calling Copilot directly with a Copilot credential (a "formal partnership"), not a client that spawns the `copilot` binary. It is evidence of GitHub's attitude, not a general license for any client to use Copilot tokens directly.

Which terms govern Copilot (as of Nov 2026)
- GitHub Terms for Additional Products and Features, Copilot section: Business/Enterprise bought directly from GitHub: "your use of GitHub Copilot is governed by the GitHub Generative AI Services Terms." Bought through Microsoft: Microsoft Product Terms. "For all other GitHub Copilot users, your use of GitHub Copilot is governed by" "Section J (AI Features) of the GitHub Terms of Service." Version effective date: August 27, 2026. [GitHub Docs](https://docs.github.com/en/site-policy/github-terms/github-terms-for-additional-products-and-features)
- That page says nothing about third-party clients or automated access in the Copilot section. [same source](https://docs.github.com/en/site-policy/github-terms/github-terms-for-additional-products-and-features)
- GitHub Copilot Product Specific Terms: "These terms have been deprecated effective 5 March 2026." Replaced by the GitHub Generative AI Services Terms. The old text said "Your use of GitHub Copilot is subject to the Acceptable Use Policies, the AI Code of Conduct, and the Required Mitigations." [GitHub customer terms](https://github.com/customer-terms/github-copilot-product-specific-terms)
- GitHub Generative AI Services Terms (Version March 2026): "your use of Generative AI Services is subject to the Acceptable Use Policies"; "you are solely responsible for any application or agent you create"; third-party products: "If you install or use any of these third-party products, you do so at your own risk." The fetch found no clause on rate limits, bulk automation, proxying, third-party clients or credential sharing. Preview features are not governed by this document. [GitHub customer terms](https://github.com/customer-terms/github-generative-ai-services-terms)
- GitHub Terms of Service (effective April 27, 2026), Section J covers ownership of Input/Output ("GitHub does not claim ownership of your Input or Output"), training opt-out, disclaimers and indemnity. It has no third-party client clause. [GitHub ToS](https://docs.github.com/en/site-policy/github-terms/github-terms-of-service)
- Same ToS, general rules: Section B.3 "Your login may only be used by one person." Section H: "You may not share API tokens to exceed GitHub's rate limitations." and "Abuse or excessively frequent requests to GitHub via the API may result in the temporary or permanent suspension" of API access. [GitHub ToS](https://docs.github.com/en/site-policy/github-terms/github-terms-of-service)
- Pre-release License Terms (they apply to previews, so possibly to ACP while in preview): "may not use the pre-release software in any active production environment, including any use to process live customer data"; "not subject to an uptime guarantee or similar service level agreement"; the software "is non-public, confidential information of GitHub". [GitHub Docs, Pre-release License Terms](https://docs.github.com/en/site-policy/github-terms/github-pre-release-license-terms)

History of unofficial Copilot token use
- Many reverse-engineered "copilot-api" proxies exist. Their READMEs say they are "not supported by GitHub, and may break unexpectedly" and repeat a GitHub Security notice that "further anomalous activity could result in temporary suspension of your Copilot access." These are third-party statements, not GitHub policy text. I found no confirmed report of a specific account ban. [copilot-api README on jsDelivr](https://cdn.jsdelivr.net/npm/copilot-api@0.7.0/README.md); [johnib/copilot-api](https://github.com/johnib/copilot-api)

Copilot CLI license (relevant if FocusIDE bundles the binary)
- Custom proprietary license: "non-exclusive, non-transferable, royalty-free license to install and run copies of the GitHub Copilot CLI." Redistribution only "in unmodified form" and "solely as part of an application or service that provides material functionality", not "on a standalone basis or as a primary product". A copy of the license and notices must be included. "Nothing in this License restricts your choice of license for your application or service." No clause on programmatic use. [github/copilot-cli LICENSE.md](https://github.com/github/copilot-cli/blob/main/LICENSE.md)

### Inferences
- Spawning the user's installed `copilot --acp` and letting the user log in through Copilot CLI itself matches a use case GitHub names ("custom frontends"). It fits FocusIDE's rule of never touching credentials. This is the lowest-risk path.
- Embedding the Copilot TUI in a terminal pane is even less exposed: it is the user running the official CLI.
- Direct use of Copilot API tokens (the copilot-api pattern) is the risky pattern. Do not do it. It would also break FocusIDE's credential rule.
- Bundling Copilot CLI looks allowed by its license (FocusIDE provides "material functionality"), and GPL-3.0 for FocusIDE is not restricted by it. A GPL lawyer may still want to check the combination, since the CLI is a separate proprietary program launched as a subprocess (aggregate, not a derivative). Safer: ask the user to install it.
- Whether the Pre-release License Terms apply to the ACP public preview is unclear. They are written for previews in general, and they bar "production" use. For a desktop client that end users opt into, this is a low practical risk, but worth a line in the UI ("preview").
- Business/Enterprise admins can disable Copilot CLI or set policies (for example enforced sandboxing overrides `--no-sandbox`, per the ACP reference). FocusIDE should surface CLI errors rather than work around them.

### Gaps
- I did not fetch the GitHub Acceptable Use Policies text directly (gh.io/aup). Any clause on automated or bulk Copilot use is unverified.
- No GitHub document found that explicitly lists permitted or forbidden third-party Copilot clients, or a partner program for third-party ACP clients.
- Billing: a secondary source says Copilot moved from premium requests to "GitHub AI Credits" on 2026-06-01. Not verified from GitHub, and how ACP prompts are metered is unknown. [dev.to post, secondary](https://dev.to/skilaai/copilot-quietly-turned-your-10-bill-into-a-meter-a-free-tool-with-170k-stars-just-won-3b3b)
- Whether ACP has left public preview by Nov 2026: the reference page fetched today still says public preview.

## 2. Copilot CLI ACP support status, hooks, custom instructions, MCP

### Takeaway
ACP is in public preview via `copilot --acp` (stdio by default, or TCP with `--port`). The CLI reads AGENTS.md and repo instruction files, supports hooks in `.github/hooks/*.json`, and supports MCP servers both from its own config and passed per session over ACP.

### Cited Findings
- Start: `copilot --acp`, optional `--stdio` (default) or `--port`. stdio: client launches it as a child process, NDJSON over stdin/stdout, one client, exits when input closes. TCP binds to 127.0.0.1 by default, accepts multiple connections. The two modes are mutually exclusive. [GitHub Docs, ACP server](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)
- Server-level options apply to all sessions and cannot be overridden per session: `--available-tools`, `--excluded-tools`, `--effort` / `--reasoning-effort` (low to max). Sandbox settings and `--sandbox`/`--no-sandbox` are honored; an enterprise policy that enforces sandboxing overrides `--no-sandbox`. [same](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)
- Methods shown in the example: `initialize`, `session/new` (takes a working directory and MCP servers), prompt, session update notifications, a `requestPermission` handler. Slash commands are sent as prompts and advertised via `available_commands_update`. Interactive commands such as `/diff`, `/resume`, `/login`, `/help` are not supported over ACP. [same](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)
- Auth over ACP: GitHub login, or BYOK through `COPILOT_PROVIDER_*` environment variables or a providers file (`COPILOT_PROVIDERS_CONFIG`). "BYOK sessions run without GitHub login." [same](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)
- Known past bug: issue #845 reported that in `--acp` mode (CLI 0.0.372) Copilot did not send `session/request_permission` for writes or shell commands and auto-approved them. Current status not verified. [github/copilot-cli issue #845](https://github.com/github/copilot-cli/issues/845)
- Custom instructions: Copilot CLI reads repository-wide instructions, path-specific instruction files and AGENTS.md; multiple files are combined, no general precedence order. `--no-custom-instructions` disables loading "from AGENTS.md and related files". Subagent custom agents need `include-custom-instructions: true` to receive repo instructions. [GitHub Docs, custom instructions](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions); [CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)
- Hooks: run custom shell commands at key points in a session; defined in `.github/hooks/*.json` or inline in `~/.copilot/settings.json`; `disableAllHooks` turns them off. [GitHub Docs, CLI config dir reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference); [Using Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/overview)
- MCP: the GitHub MCP server is preconfigured; add others with `copilot mcp add --transport http NAME URL` or `/mcp add`; `--additional-mcp-config` adds a server for one session only. [CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)

### Inferences
- FocusIDE can launch `copilot --acp` with the same ACP client it uses for `claude-agent-acp`. Plan view and undo must not rely on Copilot sending permission requests until issue #845 is confirmed fixed; FocusIDE's shadow git snapshots already cover this.
- Login has to happen outside ACP (`/login` is not supported over ACP). FocusIDE can open a terminal pane running `copilot` (or `copilot login` if that subcommand exists in the current version, unverified) and never touch the token.
- AGENTS.md support means FocusIDE's own project instructions work across Claude Code, Copilot CLI and others.

### Gaps
- Full ACP method list (session/load, session/list, set_mode, set_model) for Copilot CLI is not documented on the page fetched.
- Whether Copilot CLI hooks fire for ACP sessions is not stated.

## 3. Cursor CLI and Amazon Q / Kiro CLI (brief)

### Takeaway
Both have native ACP servers: Cursor via `agent acp`, Kiro via `kiro-cli acp`. Both docs invite third-party editors to use them. I found no third-party usage terms on the ACP pages of either product. Amazon Q Developer CLI is now Kiro CLI (the Kiro docs carry "Upgrading from Q CLI" migration guides).

### Cited Findings
Cursor
- `agent acp` is for "building custom clients and integrations"; stdio, JSON-RPC 2.0, newline-delimited JSON. Auth method `cursor_login`, or pre-auth with `agent login`, `--api-key` / `CURSOR_API_KEY`, `--auth-token` / `CURSOR_AUTH_TOKEN`. [Cursor docs, ACP](https://cursor.com/docs/cli/acp)
- Methods: `initialize`, `authenticate`, `session/new`, `session/load`, `session/prompt`, `session/cancel`; server-to-client `session/update`, `session/request_permission` (answers `allow-once`, `allow-always`, `reject-once`; "If your client does not answer permission requests, tool execution can block."). Extensions: `cursor/ask_question`, `cursor/create_plan` (blocking), `cursor/update_todos`, `cursor/task`, `cursor/generate_image` (notifications). Modes: agent, plan, ask. MCP from `.cursor/mcp.json`; "Team-level MCP servers configured through the Cursor dashboard are not supported in ACP mode." Third-party: JetBrains, Neovim (avante.nvim), Zed named; "Build or use third-party integrations for your preferred development environment." [Cursor docs, ACP](https://cursor.com/docs/cli/acp)
- Community reports of gaps: no `session/list` at one point, `session/load` "Session not found" failures, AskQuestion never offered over ACP. [Cursor forum, session/list](https://forum.cursor.com/t/acp-support-session-list-method/156222); [Cursor forum, session/load](https://forum.cursor.com/t/cursor-acp-session-load-fails-with-session-id-not-found-breaking-persistent-sessions-acpx-openclaw-acp-runtime/155516); [Cursor forum, AskQuestion](https://forum.cursor.com/t/agent-acp-never-offers-the-askquestion-tool-so-cursor-ask-question-is-never-sent/172976)

Kiro (formerly Amazon Q Developer CLI)
- `kiro-cli acp` (optionally `--agent my-agent`); stdio JSON-RPC. "`kiro-cli acp` needs a signed-in Kiro account." From 2.37 a signed-out start returns an auth-required error for everything except `initialize`. Clients that declare terminal auth get a "Log in to Kiro" option; others must run `kiro-cli login`. "Any editor supporting ACP can integrate Kiro." [Kiro docs, ACP](https://kiro.dev/docs/cli/acp/)
- Methods: `initialize`, `session/new`, `session/load`, `session/prompt`, `session/cancel`, `session/set_mode`, `session/set_model` (v2; V3 uses `session/set_config_option`). v2 extensions under `_kiro.dev/` (commands, MCP events, compaction); V3 advertises extensions in `agentCapabilities._meta.kiro.extensionMethods`. Engine selection: `kiro-cli settings acp.agentEngine v3` or `--agent-engine=v2`. [Kiro docs, ACP](https://kiro.dev/docs/cli/acp/)
- Kiro blog post on adopting ACP, and Zed's ACP registry lists Kiro CLI. [Kiro blog](https://kiro.dev/blog/kiro-adopts-acp/); [Zed ACP registry](https://zed.dev/acp/agent/kiro-cli)

### Inferences
- Both fit FocusIDE's model: spawn the vendor's own CLI, let the user sign in through the vendor's login flow, never handle the key. Cursor's `--api-key` / `CURSOR_AUTH_TOKEN` options exist, but FocusIDE should not collect them.
- Kiro's terminal-auth path (a client that declares terminal authentication) maps well to FocusIDE's terminal pane.
- Cursor's `cursor/create_plan` and `cursor/update_todos` extensions map onto FocusIDE's plan view.

### Gaps
- I did not check Cursor's Terms of Service or Kiro / AWS service terms for clauses on third-party clients. The ACP doc pages contain no usage terms.
- Kiro V3 harness details are on a separate migration page that I did not fetch.
