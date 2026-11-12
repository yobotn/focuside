# OpenAI Codex CLI: terms and technical facts for a third-party desktop app (as of 2026-11-12)

Research note: several openai.com and help.openai.com pages returned HTTP 403 to the fetch tool (Terms of Use, help article 20001542, "Unlocking the Codex harness" post). Quotes from those pages come from search snippets and are marked. developers.openai.com/codex/* now 308-redirects to learn.chatgpt.com/docs/*.

## Q1. May a third-party GUI drive Codex programmatically with a ChatGPT plan login or an API key?

### Takeaway
Yes, with documented paths. As of late 2026 OpenAI runs a formal "Sign in with ChatGPT" (SIWC) program that lets open-source and locally run apps use a user's ChatGPT plan, including through Codex app-server, with a registered OAuth client. Paid or remotely hosted apps need OpenAI approval. API key use follows normal API terms. No OpenAI enforcement against third-party Codex clients was found.

### Cited Findings
Official SIWC program (developers.openai.com, undated pages, live Nov 2026):
- Program summary quote: "Let users sign in with ChatGPT and use their ChatGPT plan for eligible AI requests in your app." [OpenAI SIWC overview](https://developers.openai.com/siwc)
- Eligibility quote: "At launch, ChatGPT plan usage is available to open-source projects, personal projects that run locally, and selected private apps." Also: "Identity-only integration is currently offered to a select group of commercial partners." [OpenAI Cookbook: Sign in with ChatGPT](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)
- Paid or hosted apps quote: "If you're building a paid or remotely hosted app, join the waitlist to request access before offering it to users." The page also tells developers to review the terms at openai.com/policies before distributing an integration. [OpenAI Cookbook](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)
- Open-source flow quotes: "your open-source app can request permission to use the user's ChatGPT plan for eligible Responses API requests." "These docs explain ChatGPT plan usage for open-source and locally hosted apps." "If you're interested in offering it in a paid or remotely hosted app, complete the interest form." [SIWC token sharing overview](https://developers.openai.com/siwc/token-sharing-open-source)
- Registration quotes: "The OAuth flow for OSS clients registers a **user-defined agent** during Sign in with ChatGPT." "each issued `client_id` is bound to the authenticated user and the workspace selected during registration." On first launch the client "opens the browser for sign-in, client registration, and consent to use the user's ChatGPT plan." Also: "This does not grant access to their ChatGPT conversations or other account context." [SIWC token sharing overview](https://developers.openai.com/siwc/token-sharing-open-source)
- Per search snippet of the sign-in page: plan-usage scopes are `offline_access resource.invoke chatgpt.tokens.use.direct` with resource `https://api.openai.com/v1`, and the OSS flow issues OAuth credentials "without a client secret or partner API key". [SIWC Registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- Codex app-server under SIWC: the app runs app-server as a child process "using an OAuth access token authorized for the user's ChatGPT plan"; "Set `ACCESS_TOKEN` in the app-server child process's environment to that value"; "Your app should read its own token file and supply the access token to the child process." "No separate Codex sign-in is required." [SIWC: Codex app-server](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)
- Preview limits for that path: app talks to app-server over stdio, app-server calls Responses with `store: false` and `stream: true`; shell tools, MCP tools and child agents work; configurations that emit Responses `tool_search` fail; "Your app must refresh the OAuth token. With the `env_key` setup, restart app-server with the renewed `ACCESS_TOKEN`, then resume the saved thread." Image generation, file search, Code Interpreter, hosted MCP/connectors are not supported. [SIWC Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- OpenAI help article "Using your ChatGPT plan in other apps and sites" (per search snippet, page 403 to fetch) lists open-source examples "OpenCode, OpenClaw, Pi by Earendil, and T3" and commercial examples including Devin and Warp. [OpenAI Help 20001542](https://help.openai.com/articles/20001542)
- Launch: DevDay announcement (reported Sept 29, 2026) that Plus and Pro subscribers can use plan allowance in third-party developer tools; sixteen launch partners including OpenCode, Devin, Amp, Warp, Conductor, Notion, Vercel; users can set weekly per-app caps in ChatGPT settings and "usage stops once the cap is hit rather than switching to the partner's own billing". [daily.dev summary of The New Stack](https://daily.dev/posts/openai-makes-sign-in-with-chatgpt-a-way-to-use-your-subscription-in-third-party-developer-tools-7ewaoo6ze), original [The New Stack](https://thenewstack.io/sign-in-with-chatgpt)

Codex auth docs (Codex's own login, not SIWC):
- "Codex supports two ways for a person to sign in when using OpenAI models": "Sign in with ChatGPT for subscription access" and "Sign in with an API key for usage-based access". "When you sign in with an API key, Codex uses standard API pricing instead of included ChatGPT plan credits." [Codex Authentication](https://learn.chatgpt.com/docs/auth) (redirect target of developers.openai.com/codex/auth)
- Automation: "Use API key authentication for programmatic Codex CLI workflows, such as CI/CD jobs." "API keys are still the recommended default for automation." "Access tokens are intended for trusted scripts, schedulers, and private CI runners." "Don't expose Codex execution in untrusted or public environments." [Codex Authentication](https://learn.chatgpt.com/docs/auth)
- Credentials: "Codex caches login details locally in a plaintext file at `~/.codex/auth.json` or in your OS-specific credential store." "treat `~/.codex/auth.json` like a password: it contains access tokens." "Don't commit it, paste it into tickets, or share it in chat." [Codex Authentication](https://learn.chatgpt.com/docs/auth)
- Admin controls: "If the active credentials don't match the configured restrictions, Codex logs the user out and exits." [Codex Authentication](https://learn.chatgpt.com/docs/auth)

app-server as the official integration surface:
- "Codex app-server is the interface Codex uses to power rich clients (for example, the Codex VS Code extension)." "Use it when you want a deep integration inside your own product." "If you are automating jobs or running Codex in CI, use the Codex SDK instead." Enterprise integrators can "contact OpenAI to get it added to a known clients list" and should set `clientInfo.name` to identify the client for the OpenAI Compliance Logs Platform. [Codex app-server docs](https://learn.chatgpt.com/docs/app-server)
- An OpenAI engineering post ("Unlocking the Codex harness") reportedly says app-server was not first designed as a stable API because the team did not expect other clients to depend on it (search summary; page returned 403). [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness)
- app-server supports an external auth mode so a parent app that owns the user's ChatGPT tokens can pass them in ("feat(app-server): support external auth mode (#10012)"), seen on a mirror of the openai/codex repo. [Mirror commit](https://git.halfakop.ru/rad/codex/src/commit/81a17bb2c1041ad1d89527911318386ce036afa9)

Terms of Use (could not fetch live page; text from search snippets):
- "You may not share your account credentials or make your account available to anyone else and are responsible for all activities that occur under your account." The restrictions list also bars automatically or programmatically extracting data or Output, interfering with the Services, and circumventing rate limits or restrictions. [OpenAI Terms](https://openai.com/api/policies/terms/); [OpenAI Terms of Use](https://openai.com/policies/row-terms-of-use/). A January 1, 2026 version appears as "previous version" in results, so a newer revision exists; current exact wording not verified.

Staff statements and partnerships:
- After Anthropic blocked subscription OAuth in third-party tools (reported Jan 9, 2026), OpenAI's Thibault Sottiaux ("Tibo") posted: "We are working with OpenCode to allow Codex users to use their Codex subscriptions and usage limits in OpenCode directly." OpenCode shipped ChatGPT Plus/Pro OAuth in v1.1.11. Secondary sources; primary X post not fetched. [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/); [MindStudio](https://www.mindstudio.ai/blog/anthropic-restricts-third-party-agents-openai-opens-codex-comparison)
- Secondary sources say a formal partnership around February 2026 covered OpenCode, OpenHands and RooCode; no primary OpenAI announcement found for that date. [MindStudio](https://www.mindstudio.ai/blog/openai-codex-anthropic-subscription-change); [besthub.dev](https://www.besthub.dev/articles/anthropic-s-model-lockdown-vs-openai-s-opencode-integration-what-it-means-for-developers-74830a2c24d8)
- A June 2026 blog post reports Sam Altman said Plus/Pro users may use their subscription with OpenClaw (paraphrase, undated, unlinked) and that pre-SIWC third-party OAuth relied on the private endpoint `chatgpt.com/backend-api/codex`, which changed and broke tools on May 27, 2026. [Zenn (ryoushin)](https://zenn.dev/ryoushin/articles/addd79c36e4e19)
- Zed (Oct 16, 2025): "We've also separately open-sourced the codex-acp adapter so you can use it outside of Zed as well." "the billing and legal/terms arrangement is directly between you and OpenAI." No OpenAI quote or stated collaboration. [Zed blog: Codex is live in Zed](https://zed.dev/blog/codex-is-live-in-zed)
- An older (about Oct 2026) guide claimed subscription login works only in OpenAI first-party clients; this conflicts with the SIWC docs and appears outdated. [SSD Nodes](https://www.ssdnodes.com/learn/codex-with-a-chatgpt-subscription)

### Inferences
- FocusIDE (GPL-3.0, local, free) fits the stated SIWC eligibility category "open-source projects". The sanctioned route for ChatGPT-plan use from FocusIDE's own chat UI is: register an OSS client through SIWC, have the user consent in the browser, and pass the access token to a Codex app-server child process via `ACCESS_TOKEN`. That flow requires FocusIDE to hold an OAuth token file, which conflicts with FocusIDE's rule of never storing credentials. The alternative that keeps the rule is to let Codex manage its own login (`codex login`, its own `auth.json` or keyring) and have FocusIDE only spawn Codex or codex-acp. Nothing found forbids a local GUI spawning the user's own Codex CLI or app-server under the user's own Codex login; OpenAI's own docs describe app-server as the surface for "deep integration inside your own product".
- Embedding the Codex TUI in a terminal pane where the user types is the same as the user running Codex, and raises no third-party-client question.
- If FocusIDE ever became paid or hosted, SIWC requires OpenAI approval (waitlist / interest form).
- Reading `~/.codex/auth.json` from FocusIDE would contradict OpenAI's guidance to treat it like a password and FocusIDE's own credential rule; avoid it.

### Gaps
- Exact current Terms of Use and Services Agreement wording (pages returned 403). No clause found that names third-party Codex clients specifically.
- Whether SIWC OSS clients must also follow SIWC "UI/UX guidelines" (page exists at /siwc/ui-ux-guidelines, not read).
- Full SIWC launch partner list (16) and whether Zed or JetBrains are among them: not confirmed.
- JetBrains Codex integration details and auth: not researched within budget.
- Primary X post URLs for Tibo and Altman statements: not retrieved.

## Q2. Codex ACP support: official or community, maintainer, status

### Takeaway
There is no OpenAI-maintained ACP adapter in Codex itself. The adapter `codex-acp` was built by Zed Industries (Oct 2025), and development moved to the `agentclientprotocol` GitHub org, where it is now built on Codex app-server. The original Zed repo was archived Jul 22, 2026.

### Cited Findings
- zed-industries/codex-acp: "Development has moved to agentclientprotocol/codex-acp." "This repository was archived by the owner on Jul 22, 2026. It is now read-only." License Apache-2.0. Auth listed: "ChatGPT subscription (requires paid subscription and doesn't work in remote projects)", CODEX_API_KEY, OPENAI_API_KEY. New installs should use `@agentclientprotocol/codex-acp`. [GitHub zed-industries/codex-acp](https://github.com/zed-industries/codex-acp)
- agentclientprotocol/codex-acp: the adapter "starts the Codex App Server, translates ACP requests into Codex operations." Auth: ChatGPT login (browser; can be hidden with `NO_BROWSER=1`), API key via `CODEX_API_KEY` or `OPENAI_API_KEY`, or a custom OpenAI-compatible gateway when the client opts in. Apache 2.0. The npm package "includes a compatible `@openai/codex` dependency." No claim of OpenAI involvement. [GitHub agentclientprotocol/codex-acp](https://github.com/agentclientprotocol/codex-acp)
- Zed installs codex-acp automatically the first time a Codex thread is created; Zed agent API key settings are not used for Codex billing or auth. [Zed ACP agent page](https://zed.dev/acp/agent/codex); [AlternativeTo](https://alternativeto.net/news/2025/10/zed-editor-integrates-openai-codex-via-acp-and-open-sources-adapter)
- Community forks exist (sagemathinc for CoCalc, noom, cola-io); status not verified. [sagemathinc/codex-acp](https://github.com/sagemathinc/codex-acp)

### Inferences
- With codex-acp, ChatGPT login is performed by Codex itself (Codex stores the token), so a client like FocusIDE never touches credentials. This matches how Zed uses it.
- Because codex-acp now sits on app-server, its turn and item events derive from the app-server event stream.

### Gaps
- Latest release version and date of agentclientprotocol/codex-acp (release list did not render).
- Named maintainers in the agentclientprotocol org.

## Q3. Codex extension surfaces and which report turn start/end

### Takeaway
Codex has AGENTS.md, config.toml, MCP (client and server), a `notify` command, a full hooks system (on by default, Claude-Code-like events), and app-server JSON-RPC. Turn boundaries are reported by app-server notifications (`turn/started` style events and `turn/completed`), hooks (`UserPromptSubmit`, `Stop`, `Interrupt`, `SessionStart`/`SessionEnd`), and `notify` (turn end only).

### Cited Findings
- Hooks: "Hooks are enabled by default." Disable with `[features] hooks = false`. Locations: `~/.codex/hooks.json`, `~/.codex/config.toml`, `<repo>/.codex/hooks.json`, `<repo>/.codex/config.toml`, plus plugins and managed `requirements.toml`. "Non-managed hooks must be reviewed and trusted before they run"; trust is tied to the hook hash, changed hooks are skipped until re-trusted (`/hooks` in the CLI). Project-local hooks load only when the project `.codex/` layer is trusted. [Codex hooks docs](https://learn.chatgpt.com/docs/hooks.md)
- Hook events: SessionStart ("When a session or subagent starts", `source` startup/resume/clear/compact/fork), SessionEnd ("When the main thread ends"), SubagentStart, SubagentStop, PreToolUse, PermissionRequest, PostToolUse, PreModelRequest, PostModelRequest (with `token_usage`), PreCompact, PostCompact, UserPromptSubmit (`turn_id`, `prompt`), Stop (`turn_id`, `stop_hook_active`, `last_assistant_message`), Interrupt ("When you interrupt an active turn"). Common payload: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`; `turn_id` is a "Codex-specific extension". [Codex hooks docs](https://learn.chatgpt.com/docs/hooks.md)
- Older third-party guides (Mar to Apr 2026) said hooks needed `codex_hooks = true`; official docs now say on by default and `codex_hooks` still works as an alias (search summary). [Daniel Vaughan hooks deep dive](https://codex.danielvaughan.com/2026/03/26/codex-cli-hooks-deep-dive); [Codex hooks docs](https://developers.openai.com/codex/hooks)
- notify: a root-level `notify = ["cmd", "arg"]` in `~/.codex/config.toml`; Codex appends one JSON argument; `type` is currently only `agent-turn-complete`; fields include `thread-id`, `turn-id`, `cwd`, `input-messages`, `last-assistant-message`, and a newer `client` field (`codex-tui`, or the app-server `clientInfo.name`). `[tui] notifications` is separate and also supports `approval-requested`. Sources are mirrored docs/config.md and community posts, not the live official page. [Mirror of codex docs/config.md](https://git.halfakop.ru/mirrors/codex/src/commit/679de088a30d3e793f8c4115ca81bb06ec53e5f7/docs/config.md); [Daniel Vaughan notification pipeline](https://codex.danielvaughan.com/2026/04/13/codex-cli-notification-pipeline-osc9-hooks-alerts/)
- app-server: stdio JSONL by default; "WebSocket transport is experimental and unsupported"; some methods gated behind `experimentalApi` capability. After `turn/start`, clients read `item/started`, `item/completed`, `item/agentMessage/delta`; "The server emits `turn/completed` with final status when the model finishes". [Codex app-server docs](https://learn.chatgpt.com/docs/app-server)
- SIWC note: for turns "only `completed` indicates success; `failed` and `interrupted` do not." [SIWC: Codex app-server](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)
- Config can point model providers at command-backed auth (`[model_providers.<id>.auth]`), env vars (`env_key`), or `requires_openai_auth = true` for proxies. [Codex Authentication](https://learn.chatgpt.com/docs/auth)

### Inferences
- For an embedded Codex TUI in a FocusIDE terminal pane, FocusIDE can learn turn start/end without touching credentials by installing a hook (`UserPromptSubmit` for start, `Stop` / `Interrupt` for end, with `turn_id`) or a `notify` command (end only). A user hook needs one-time trust in Codex (`/hooks`), and a project-level hook needs the project trusted. Using `-c` config overrides or a `CODEX_HOME` would avoid editing the user's files, but whether hooks can be passed by `-c` was not verified.
- For the chat UI path, app-server (directly or via codex-acp) gives structured turn and item events and is the documented integration surface.

### Gaps
- Official live text for `notify` (only mirrors and community posts found).
- Whether `codex mcp-server` mode and the Codex SDK remain supported in Nov 2026 and their auth rules (not checked in this pass; app-server docs point CI users to the SDK).
- AGENTS.md behavior not re-verified (widely known: Codex reads AGENTS.md as project instructions).

## Q4. Enforcement events or rule changes

### Takeaway
No OpenAI enforcement against third-party Codex or ChatGPT-plan clients was found. The rule changes run toward openness: welcoming OpenCode (Jan 2026), reported partnerships (Feb 2026), and the formal SIWC program at DevDay (late Sept 2026). The main disruption was a technical break of the private backend on May 27, 2026.

### Cited Findings
- Jan 2026: OpenAI staff said they were working with OpenCode to allow Codex subscriptions in OpenCode, after Anthropic's block. [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/)
- May 27, 2026: change to `chatgpt.com/backend-api/codex` broke third-party OAuth users; no enforcement reported; the author states OpenAI does not prohibit third-party OAuth use (unsourced claim). [Zenn](https://zenn.dev/ryoushin/articles/addd79c36e4e19)
- Sept 2026: SIWC program with per-app weekly caps and an approval gate for paid or hosted apps. [daily.dev / The New Stack](https://daily.dev/posts/openai-makes-sign-in-with-chatgpt-a-way-to-use-your-subscription-in-third-party-developer-tools-7ewaoo6ze); [OpenAI Cookbook](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)
- Billing change: one pricing guide says that on April 2, 2026 OpenAI moved Plus, Pro, Business and new Enterprise plans from per-message to token-based credit billing, and added a $100 Pro tier. Secondary source. [eesel.ai](https://www.eesel.ai/blog/codex-pricing)
- Jul 22, 2026: zed-industries/codex-acp archived, moved to agentclientprotocol org. [GitHub](https://github.com/zed-industries/codex-acp)

### Inferences
- The SIWC program likely signals a shift from tolerating unofficial reuse of the private backend to a registered-client model. Unregistered apps that reuse Codex tokens against `chatgpt.com/backend-api/codex` may face future restrictions. Apps that just run the official Codex binary (TUI, app-server, codex-acp) with Codex's own login do not depend on that.

### Gaps
- No primary OpenAI statement found on unregistered clients that reuse Codex CLI tokens.
- No reports of account bans found, but absence of reports is not proof of absence.
