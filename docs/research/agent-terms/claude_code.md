# Claude Code: terms and technical facts for a third-party desktop app (FocusIDE), as of 2026-11-12

Research date: 2026-11-12. Pages fetched live on that date unless a date is given. Quotes are verbatim from the fetched page. "Inference" items are my interpretation, not Anthropic text.

## Q1. What do Anthropic's docs and terms say about third-party products on Claude Code or the Agent SDK, subscription login vs API keys? Is there an approval process?

### Takeaway
Anthropic's current docs forbid third-party developers from offering claude.ai login or routing requests through Free/Pro/Max/Ultra credentials "on behalf of their users", and tell developers (including Agent SDK developers) to use API keys. The same page explicitly carves out "an end user signing in to the unmodified Claude Code binary with their own Claude subscription". The only approval path named is "Unless previously approved" (Agent SDK page) and "contact sales"; there is no public application form.

### Cited Findings
- Claude Code legal page, "Authentication and credential use": "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications." (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page: "Developers building products or services that interact with Claude's capabilities, including those using the Agent SDK, should use API key authentication through Claude Console or a supported cloud provider. Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens - sign-in to a Claude account must complete through Anthropic's own flow." (the original uses a dash at that point) (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page, carve-out: "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code as described under *Can customers offer Claude Code in their products?* above." (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page: "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice." and "For questions about permitted authentication methods for your use case, please contact sales." (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page, "Can customers offer Claude Code in their products?": "Unless we've mutually agreed otherwise, preinstalling or running Claude Code in your products or services (e.g. in hosted sandboxes or other agent infrastructure) requires agreeing to our Commercial Terms of Service and complying with the conditions below: The Claude Code binary must not be modified. Claude Code must be installed and run as published by Anthropic, and customers may not remove, disable, or restrict any authentication method built into it (including methods that permit signing in with a Claude account or the user's own API key). Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key, Claude subscription plan credentials, or 3P inference provider credential ... That usage is billed directly to the end user under their own agreement with Anthropic." (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page, naming: "You can accurately say, in plain text, that your product has Claude Code preinstalled or that it runs Claude Code. But you can't use the Claude Code or Anthropic names or logos as part of your own product, feature, or company name, in your own logo, or in a way that suggests Anthropic built, endorses, or is partnered with your product." (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Same page, usage policy: "Advertised usage limits for Pro, Max, and Ultra plans assume ordinary, individual usage of Claude Code and the Agent SDK." License line: Commercial Terms "for Team, Enterprise, and Claude API users"; Consumer Terms "for Free, Pro, Max, and Ultra users". (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Agent SDK overview note: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK. Use the API key authentication methods described in the Quickstart instead." (Source: [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview))
- Agent SDK branding: Allowed "Claude Agent", "Claude" (inside an "Agents" menu), "{YourAgentName} Powered by Claude". Not permitted: "'Claude Code' or 'Claude Code Agent'" and "Claude Code-branded ASCII art or visual elements that mimic Claude Code". "Your product should maintain its own branding and not appear to be Claude Code or any Anthropic product." (Source: [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview))
- Agent SDK license: "Use of the Claude Agent SDK is governed by Anthropic's Commercial Terms of Service, including when you use it to power products and services that you make available to your own customers and end users". (Source: [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview))
- Older wording (Feb 2026, as quoted by The Register, no longer present verbatim on the live page): "Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service - including the Agent SDK - is not permitted and constitutes a violation of the Consumer Terms of Service." (Source: [The Register, 2026-02-20](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access))
- Consumer Terms (page shows effective date "Oct 8, 2025"), Section 3 item 7 prohibits access "through automated or non-human means, whether through a bot, script, or otherwise" "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it". Section 2: "You may not share your Account login information, Anthropic API key, or Account credentials with anyone else." Section 3 also bars "bypassing any of our systems or protective measures". The Consumer Terms do not mention OAuth tokens or harnesses. (Source: [Consumer Terms](https://www.anthropic.com/legal/consumer-terms)); The Register says Section 3.7 was "unchanged since February 2024" (Source: [The Register](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access))
- Billing status of Agent SDK and `claude -p` on a subscription: support article update "June 15, 2026: We've paused the previously-announced changes to Claude Agent SDK usage"; Agent SDK, `claude -p` and third-party app usage still draw from subscription limits. Update "October 7, 2026": Max, Ultra and Team plans include monthly API credits that cover the Agent SDK and `claude -p` when used with an API key from a linked Console org; "If you're signed in with your Claude plan, the Agent SDK and `claude -p` still draw from your subscription limits." (Source: [Use the Claude Agent SDK with your Claude plan](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan)) (paraphrased by fetch tool; short quotes verbatim)
- The paused plan (announced around May 13-14, 2026) would have moved Agent SDK, `claude -p` and third-party/ACP usage to a separate "Agent SDK credit" ($20 Pro, $100 Max 5x, $200 Max 20x) from June 15, 2026. (Source: [Zed blog, 2026-05-14, updated 2026-06-16](https://zed.dev/blog/anthropic-subscription-changes); [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/))

### Inferences
- The live legal page distinguishes two things: (a) a third-party app offering claude.ai login itself or using a user's subscription token in its own requests (forbidden), and (b) a user signing in to the unmodified `claude` binary through Anthropic's own flow (explicitly not restricted). Pane mode (PTY running the user's own installed `claude`, user logs in via `/login`, FocusIDE never touches `.credentials.json`) fits (b) on the plain text.
- The "preinstalling or running Claude Code in your products" clause says this "requires agreeing to our Commercial Terms". It is written for platform/hosting businesses. Whether a free GPL desktop app that launches a user-installed binary counts as "running Claude Code in your products" is not addressed. Conservative reading: do not bundle the binary, launch the user's own install, do not hide or restrict any login method, do not pay for usage.
- The current ACP mode (claude-agent-acp, which uses the Agent SDK) is closer to the "Agent SDK" language. The SDK note says third parties may not "offer claude.ai login ... including agents built on the Claude Agent SDK" "unless previously approved". Yet the support article (Oct 2026) treats SDK use while "signed in with your Claude plan" as an ordinary, billed-to-subscription case. These two sources are in tension; the docs give no clear ruling on an open-source SDK client where the user logs in through Claude Code's own flow.
- Naming: FocusIDE may say it "runs Claude Code" in plain text but must not use Claude Code marks in its name/logo or mimic Claude Code visuals in its own UI. In pane mode the real TUI is shown, which is Anthropic's own UI, not mimicry.

### Gaps
- No public approval form or process found. Only "Unless previously approved" and "contact sales".
- Commercial Terms and Usage Policy pages were not fetched in this session; no clause specific to OAuth was found in them via secondary sources.
- No Anthropic statement found on open-source, non-commercial clients specifically.

## Q2. Does running the user's own `claude` CLI inside a third-party terminal emulator count as allowed? Headless (`claude -p`, stream-json) with subscription login?

### Takeaway
Running the unmodified binary with the user's own login is explicitly not restricted by the legal page, and Zed and JetBrains both publicly direct subscription users to run `claude` in their built-in terminal for exactly that reason. `claude -p` with a subscription is documented and currently draws from subscription limits, but third-party products driving it programmatically sit closer to the "Agent SDK / developers" language.

### Cited Findings
- Legal page carve-out quoted in Q1: the restrictions do not "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription". (Source: [Legal and compliance](https://code.claude.com/docs/en/legal-and-compliance))
- Zed: "When the official `claude` CLI runs in the terminal, it uses your subscription's limits, not the new credit." (offered as the way to keep subscription billing under the paused June 2026 plan) (Source: [Zed blog](https://zed.dev/blog/anthropic-subscription-changes))
- JetBrains Air docs: "A Claude subscription cannot be used to authorize Claude Agent in the chat surface." "To use Claude Agent, select JetBrains AI or configure Anthropic API-key authentication with API billing." "To use your Claude subscription, run Claude Code in the terminal surface" (opened as a full-screen editor tab). (Source: [JetBrains Air authorization](https://www.jetbrains.com/help/air/air-in-ides-authorization.html.md))
- Agent SDK overview: "To drive the same agent loop from a language other than Python or TypeScript, run the CLI as a subprocess with the `-p` flag and `--output-format json`." (Source: [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview))
- Auth precedence: in `-p` mode, "the key is always used when present" (ANTHROPIC_API_KEY); subscription OAuth from `/login` is lowest precedence (7th). `claude setup-token` produces a one-year subscription OAuth token "For CI pipelines, scripts, or other environments where interactive browser login isn't available". Bare mode does not read `CLAUDE_CODE_OAUTH_TOKEN`. (Source: [Authentication](https://code.claude.com/docs/en/authentication))
- Credentials are stored at `%USERPROFILE%\.claude\.credentials.json` on Windows, `~/.claude/.credentials.json` on Linux, macOS Keychain on macOS. (Source: [Authentication](https://code.claude.com/docs/en/authentication))
- Support article: Agent SDK and `claude -p` "still draw from your subscription limits" when signed in with a Claude plan (Oct 7, 2026 update). (Source: [support.claude.com 15036540](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan))

### Inferences
- Pane mode matches the pattern that Zed and JetBrains (both large vendors) publicly recommend for subscription users, which is strong practical evidence that it is tolerated.
- To stay within the carve-out: do not pass `ANTHROPIC_API_KEY` or `CLAUDE_CODE_OAUTH_TOKEN` into the PTY env on the user's behalf, do not read `.credentials.json`, do not script `/login`, do not inject keystrokes that automate the session beyond what the user typed (automated driving starts to look like "automated or non-human means" in Consumer Terms 3.7).
- A hook-based plugin installed by the user that writes files is ordinary Claude Code extensibility, not credential use.

### Gaps
- No Anthropic statement found that names terminal-embedding by a third-party app explicitly. The legal-page carve-out is the closest text.
- No statement found on whether a third-party UI driving `claude -p --output-format stream-json` with the user's subscription counts as "routing requests through plan credentials on behalf of users".

## Q3. Enforcement events and Anthropic statements

### Takeaway
Anthropic enforced server-side from January 9, 2026 against tools that reused subscription OAuth while spoofing Claude Code (OpenCode, others), clarified docs in February, sent legal demands to OpenCode in March, and on April 4, 2026 ended subscription coverage for third-party harnesses like OpenClaw. A June 2026 billing split for SDK/ACP usage was announced then paused.

### Cited Findings
- Jan 9, 2026: subscription OAuth tokens stopped working outside Claude Code and Claude.ai; OpenCode, OpenClaw and others broke without announcement. (Source: [falcao.org, 2026-05-24](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/); [piunikaweb, 2026-01-09](https://piunikaweb.com/2026/01/09/anthropic-reportedly-blocking-non-official-claude-api-connections/))
- Thariq Shihipar (Anthropic), January thread: "Third-party harnesses using Claude subscriptions create problems for users and are prohibited by our Terms of Service," and "They generate unusual traffic patterns without any of the usual telemetry that the Claude Code harness provides". He also said Anthropic had acted against tools "spoofing the Claude Code harness". (Source: [The Register, 2026-02-20](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access); [VentureBeat](https://venturebeat.com/technology/anthropic-cracks-down-on-unauthorized-claude-usage-by-third-party-harnesses))
- Feb 17-20, 2026: Anthropic updated the Claude Code legal docs to state OAuth use in other products "including the Agent SDK" is not permitted. (Source: [The Register](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access))
- Mar 19, 2026: legal demands to OpenCode; maintainer merged PR #18186 removing Anthropic OAuth (titled "anthropic legal requests"). (Source: [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/)) Secondary source only; PR not verified directly.
- Apr 4, 2026: Pro/Max no longer cover third-party harnesses (OpenClaw named); Boris Cherny (Head of Claude Code): "subscriptions weren't built for the usage patterns of these third-party tools." One-time transition credits reported. (Source: [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/); [The New Stack](https://thenewstack.io/anthropic-claude-harness-restrictions/))
- May 13-14, 2026: separate Agent SDK credit announced for June 15; June 15-16, 2026: paused, "Anthropic says they are revising the plan and will share advance notice before any future change takes effect." (Source: [Zed blog](https://zed.dev/blog/anthropic-subscription-changes); [support.claude.com](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan))

### Inferences
- The enforced pattern targets tools that use the subscription token directly against the API (impersonating Claude Code), not tools that launch the real binary. Pane mode and the ACP adapter (which runs the real Claude Code via the SDK) were not the stated targets.

### Gaps
- No primary Anthropic blog post or tweet URL for the Thariq and Cherny quotes was retrieved; quotes come via The Register and falcao.org.
- No evidence found of enforcement against ACP clients that use claude-agent-acp with the user's own Claude Code login.

## Q4. How other editors do it

### Takeaway
Anthropic's own VS Code and JetBrains plugins and Desktop use claude.ai OAuth. Zed runs Claude Code via ACP with the user logging in through Claude Code's own `/login` flow, and no Zed-Anthropic arrangement is disclosed. JetBrains Air blocks subscription in its chat surface and sends subscription users to a terminal tab running Claude Code.

### Cited Findings
- Anthropic's own surfaces: `ANTHROPIC_API_KEY` etc. "apply to the CLI and the surfaces that wrap it, including the VS Code extension, the Agent SDK, and GitHub Actions"; "Terminal, VS Code extension, and Agent SDK logins" verify org restrictions for claude.ai logins. (Source: [Authentication](https://code.claude.com/docs/en/authentication))
- Zed docs: open a Claude Code thread and run `/login`, then authenticate via API key or "Log in with Claude Code" for Pro/Max; since v0.202.7 "authentication to Zed's Claude Code installation is decoupled entirely from Zed's agent". (Source: [Zed external agents docs mirror](https://git.secluded.site/zed/blob/v0.223.5/docs/src/ai/external-agents.md?raw=1)) (via search summary)
- Zed's May 2026 post describes no partnership with Anthropic. (Source: [Zed blog](https://zed.dev/blog/anthropic-subscription-changes))
- JetBrains Air: subscription not allowed in chat surface; use JetBrains AI or API key; or run Claude Code in the terminal surface. (Source: [JetBrains Air authorization](https://www.jetbrains.com/help/air/air-in-ides-authorization.html.md)). A JetBrains blog (Aug 2026) reportedly describes Claude subscriptions in Air via Claude's native login without Air seeing credentials, which conflicts with the help page; the blog returned HTTP 403 and could not be verified. (Source: [JetBrains Air blog, Aug 2026](https://blog.jetbrains.com/air/2026/08/new-in-air-claude-subscriptions-multiproject-view-and-improved-markdown/))
- JetBrains AI Assistant: Claude Agent via JetBrains AI subscription, BYOK API key, or Anthropic Console login. (Source: [AI Assistant 2026.2 help](https://www.jetbrains.com/help/ai-assistant/2026.2/activate-agents.html))
- JetBrains and Zed co-develop ACP (announced Oct 2025). (Source: [Tessl, 2025-10-23](https://tessl.io/blog/jetbrains-joins-agent-client-protocol-to-open-ai-coding-agents-to-any-ide/))
- Conductor: a user feature request says Conductor authenticates via OAuth to a single Anthropic account; no Anthropic statement about Conductor found. (Source: [claudeissues.com 46090](https://claudeissues.com/issue/46090-feature-request-per-workspace-account-subscription-binding-in-conductor))

### Inferences
- The industry pattern in late 2026: chat UIs built on the SDK use API keys or vendor billing when cautious (JetBrains), or rely on Claude Code's own login inside the adapter (Zed); terminal embedding is the universally offered subscription path.

### Gaps
- Cursor's Claude Code integration and auth were not researched (no reliable source found within budget).
- No disclosed Anthropic agreements with Zed, JetBrains or Conductor found.

## Q5. Claude Code extension surfaces (hooks, plugins, skills, output styles, status line, settings)

### Takeaway
Hooks and plugins are fully documented. The Stop hook now receives `last_assistant_message` directly, and UserPromptSubmit receives `prompt`, so a plugin can write plan/summary files without parsing transcripts. Some plugin parts (themes, monitors, evals) are marked experimental.

### Cited Findings
- Hook events listed (Nov 2026): SessionStart, Setup, UserPromptSubmit, UserPromptExpansion, PreToolUse, PermissionRequest, PermissionDenied, PostToolUse, PostToolUseFailure, PostToolBatch, Notification, MessageDisplay, SubagentStart, SubagentStop, TaskCreated, TaskCompleted, Stop, StopFailure, TeammateIdle, InstructionsLoaded, ConfigChange, CwdChanged, DirectoryAdded, FileChanged, WorktreeCreate, WorktreeRemove, PreCompact, PostCompact, PreModelSwitch, PostModelSwitch, Elicitation, ElicitationResult, SessionEnd. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- Common input fields: `session_id`, `prompt_id`, `transcript_path`, `cwd`, `scratchpad_dir` (v2.1.257+), `permission_mode`, `effort`, `hook_event_name`. "`transcript_path`: conversation JSON. It is written asynchronously and may lag the current turn." (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- UserPromptSubmit: "receive the `prompt` field containing the submitted text. Pasted content that collapsed to a `[Pasted text #N]` placeholder arrives expanded in place." Also `session_title` when set. Can block with `decision: "block"`; stdout or `hookSpecificOutput.additionalContext` adds context. "It also fires on turns Claude Code starts on its own." (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- Stop: "Runs when the main Claude Code agent has finished responding. Does not run if the stoppage occurred due to a user interrupt. API errors fire StopFailure instead." Input: "`stop_hook_active`, `last_assistant_message`, `background_tasks`, and `session_crons`". "The `last_assistant_message` field contains the text content of Claude's final response, so hooks can access it without parsing the transcript file. ... the transcript file isn't guaranteed to include the final message at Stop time on all versions." 8-consecutive-continuation cap, configurable via `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- SubagentStop input example includes `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path`, `last_assistant_message`, `background_tasks`, `session_crons`. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- SessionStart `source`: "startup", "resume", "clear", "compact", "fork"; optional `model`, `agent_type`, `session_title`. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- Hook types: `command`, `http`, `mcp_tool`, `prompt`, `agent` (agent marked "Experimental"). Default timeout 600s for command/http/mcp_tool. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks))
- Plugin hooks live in `hooks/hooks.json` and "merge with user and project hooks"; variables `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}` (persists across updates, `~/.claude/plugins/data/<id>/`), `${CLAUDE_PROJECT_DIR}` are exported to hook processes. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks); [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))
- Plugin manifest: optional `.claude-plugin/plugin.json`; components: skills, commands, agents, hooks, mcpServers, lspServers, outputStyles, workflows, `bin/`, `settings.json` (only `agent` and `subagentStatusLine` take effect); `experimental` holds themes, monitors, evals "whose manifest shape may still change". (Source: [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))
- "A `CLAUDE.md` at the plugin root isn't loaded as context ... To include instructions that load into Claude's context, put them in a skill." (Source: [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))
- Reserved names: plugin names starting `claude-`, `anthropic-`, or equal to `claude-code` etc. are validation errors; `claude` as a whole word elsewhere is a warning. (Source: [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))
- Output styles are a plugin component (`output-styles/`). Plugins can be distributed via marketplaces (`marketplace.json`) and submitted to "Anthropic's directory". (Source: [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))
- In `-p` mode: plugin monitors "don't start with the `-p` flag"; PreToolUse `"defer"` is honored only with `-p`; MessageDisplay runs once per assistant message in non-interactive runs. (Source: [Hooks reference](https://code.claude.com/docs/en/hooks); [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference))

### Inferences
- For pane mode: a FocusIDE plugin (name it without a reserved prefix, for example `focuside`) with UserPromptSubmit (record turn start, `prompt`, `session_id`) and Stop (write `last_assistant_message` to a summary file in `${CLAUDE_PLUGIN_DATA}` or the project) is fully supported by documented fields. Instructions should go in a skill or via UserPromptSubmit `additionalContext`, not a plugin CLAUDE.md.
- Write additionalContext as factual statements; the docs warn imperative "system command" text "can trigger prompt-injection defenses".

### Gaps
- Status line docs and settings scopes (user, project, local, managed) were not fetched in this session; the plugin docs mention `subagentStatusLine` and `enabledPlugins` but not the main status line schema.
- No explicit API stability or deprecation policy for hooks found; fields carry version requirements (for example v2.1.257+).

## Q6. Is claude-agent-acp official or community? Maintainer, license

### Takeaway
It lives under the `agentclientprotocol` GitHub org (the ACP project, started by Zed), not under `anthropics`. License Apache-2.0. It wraps the official Claude Agent SDK. It is not an Anthropic product.

### Cited Findings
- Repo `agentclientprotocol/claude-agent-acp`, description "Use Claude Agent SDK from any ACP client"; uses the "official Claude Agent SDK"; license Apache-2.0; no CLA. Supports `CLAUDE_CODE_EXECUTABLE` to use an existing Claude CLI; smoke test "needs a Claude login". Contains JetBrains AIR-specific extensions; v2 protocol support labeled "Experimental". The README does not name individual maintainers. (Source: [GitHub claude-agent-acp](https://github.com/agentclientprotocol/claude-agent-acp))
- JetBrains partnered with Zed to co-develop ACP (Oct 2025). (Source: [Tessl](https://tessl.io/blog/jetbrains-joins-agent-client-protocol-to-open-ai-coding-agents-to-any-ide/))

### Inferences
- Because the adapter is built on the Agent SDK, the Agent SDK terms (Commercial Terms; "Unless previously approved" note on claude.ai login) are the most relevant text for FocusIDE's current ACP mode. Apache-2.0 is compatible with distributing it alongside a GPL-3.0 app.
- The underlying Claude Code binary used by the SDK is Anthropic proprietary software under Anthropic's terms, separate from the adapter's Apache license.

### Gaps
- Named maintainers and whether Zed or JetBrains staff maintain it were not confirmed from the page.
- Whether the adapter version FocusIDE pins bundles the Claude Code binary or uses a user-installed one was not checked here (relevant to the "must not be modified" and "preinstalling" clauses).
