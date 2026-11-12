# Challenge 1: adversarial check of the Anthropic claims (Claude Code, Agent SDK, adapter)

Checked 2026-11-12. Every page below was fetched live on that date unless marked "secondary". Where a quoted source uses an em dash, it is shown here as " - ". Not legal advice.

Summary table

| # | Claim | Verdict |
| --- | --- | --- |
| 1 | Legal page forbids third-party Claude.ai login and plan-credential routing, but carves out the user signing in to the unmodified binary | CONFIRMED |
| 2 | "Products" clause requires Commercial Terms; does it cover a free GPL app that spawns the user's own `claude`? | PARTLY (quote correct; scope is not stated, and FocusIDE has a second hook into Commercial Terms the notes missed) |
| 3 | Agent SDK "Unless previously approved" note and branding rules | CONFIRMED |
| 4 | Support article 15036540 updates, and whether it rules on third-party GUIs | CONFIRMED on text; the article does NOT rule on third-party GUIs |
| 5 | Adapter license Apache-2.0 vs registry "proprietary"; bundling of Claude Code | PARTLY (both labels are real; release archives DO bundle the Claude CLI; the adapter exe also embeds the proprietary Agent SDK) |
| 6 | Hooks and plugin facts | CONFIRMED (reserved-name list is wider than stated) |
| 7 | Enforcement timeline; none targeted tools running the real binary | PARTLY / CONTRADICTED on the last point |
| 8 | Zed and JetBrains tell subscription users to run `claude` in the terminal | PARTLY (JetBrains yes, current; Zed only in a blog post about the paused billing plan) |

---

## Claim 1. Legal page: no third-party Claude.ai login, but end-user sign-in to the unmodified binary is not prevented

Verdict: CONFIRMED.

URL: https://code.claude.com/docs/en/legal-and-compliance

Quotes (section "Authentication and credential use"):

> "Developers building products or services that interact with Claude's capabilities, including those using the Agent SDK, should use API key authentication through Claude Console or a supported cloud provider. Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens - sign-in to a Claude account must complete through Anthropic's own flow."

> "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code as described under *Can customers offer Claude Code in their products?* above."

> "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications."

> "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice."

What to watch: the carve-out sits next to the hosting clause, so its natural reading is "a platform may run Claude Code and let users log in to it". It does not say that anything a third-party app does around the binary is fine. The "ordinary use" wording in the OAuth bullet and "ordinary, individual usage" in the usage policy section ("Advertised usage limits for Pro, Max, and Ultra plans assume ordinary, individual usage of Claude Code and the Agent SDK.") are the hooks Anthropic can use against automated driving.

## Claim 2. "Running Claude Code in your products" requires Commercial Terms. Who is covered?

Verdict: PARTLY. The quote is accurate. The page does not define "products or services", "customers", or say whether free or open source software counts.

URL: https://code.claude.com/docs/en/legal-and-compliance

> "Unless we've mutually agreed otherwise, preinstalling or running Claude Code in your products or services (e.g. in hosted sandboxes or other agent infrastructure) requires agreeing to our Commercial Terms of Service and complying with the conditions below:"
> "The Claude Code binary must not be modified. Claude Code must be installed and run as published by Anthropic, and customers may not remove, disable, or restrict any authentication method built into it (including methods that permit signing in with a Claude account or the user's own API key)."
> "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key, Claude subscription plan credentials, or 3P inference provider credential ..."
> "Claude Code remains governed by Anthropic's standard terms ... regardless of the platform through which it is accessed."

Analysis:
- The example ("hosted sandboxes or other agent infrastructure") and the word "customers" point at businesses that host or ship Claude Code. Nothing says "free" or "open source" is exempt. A desktop app that only launches the user's own installed `claude` arguably does not "preinstall" it, and "running Claude Code in your products" is a plain-text fit for what pane mode does. So the safe assumption is that the conditions apply: unmodified binary, every login method left available, no paying for usage.
- Restriction risk the notes did not flag: "may not remove, disable, or restrict any authentication method built into it". FocusIDE's current ACP mode refuses to start when Claude Code is signed in with a plan (`apps/desktop/src/lib/i18n/en.ts` key `access.signedOut`, `ClaudeAccess.svelte`; status `claudeAccess`). That restricts a login method of the binary as FocusIDE runs it. It is a conservative choice for billing, but read literally it conflicts with this condition. JetBrains Air does the same in its chat surface, so it is a common pattern, but the text does not carve it out.
- Second hook into Commercial Terms that the notes missed: FocusIDE ships the `claude-agent-acp` executable. That executable is built with `bun build --compile` and embeds `@anthropic-ai/claude-agent-sdk` (see claim 5). The Agent SDK license reads "Use of the Claude Agent SDK is governed by Anthropic's Commercial Terms of Service, including when you use it to power products and services that you make available to your own customers and end users" (https://code.claude.com/docs/en/agent-sdk/overview). So FocusIDE, as a distributor of an SDK-based component, is under Commercial Terms for ACP mode whatever the answer for pane mode.

## Claim 3. Agent SDK note and branding

Verdict: CONFIRMED.

URL: https://code.claude.com/docs/en/agent-sdk/overview

> "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK. Use the API key authentication methods described in the Quickstart instead."

Branding:
> Allowed: "\"Claude Agent\", preferred for dropdown menus"; "\"Claude\", when within a menu already labeled \"Agents\""; "\"{YourAgentName} Powered by Claude\", if you have an existing agent name"
> Not permitted: "\"Claude Code\" or \"Claude Code Agent\""; "Claude Code-branded ASCII art or visual elements that mimic Claude Code"
> "Your product should maintain its own branding and not appear to be Claude Code or any Anthropic product."

License line adds a clause the notes cut off: "... except to the extent a specific component or dependency is covered by a different license as indicated in that component's LICENSE file."

Note: the branding section addresses "partners integrating the Claude Agent SDK". For pane mode it does not apply directly, but the legal page naming rule does: plain-text "runs Claude Code" is fine; Claude Code in product or feature names is not. A FocusIDE menu or tab label "Claude Code" is a feature name and is risky. "Claude" inside an "Agents" menu is the safe label.

## Claim 4. Support article 15036540

Verdict: CONFIRMED on the text. The article does NOT say whether a third-party GUI driving the user's own `claude` with a plan login is allowed.

URL: https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan (page metadata dateModified 2026-10-30)

The live article now contains only two callouts (extracted from the HTML):

> "Update October 7, 2026: Claude Max, Ultra, and Team plans now include monthly API credits. They cover the Claude API, Claude Managed Agents, and the Claude Agent SDK and claude -p when you run them with an API key from your linked Claude Console organization. Nothing changes for your plan: when you're signed in with your Claude plan, the Claude Agent SDK and claude -p still draw from your subscription limits."

> "Update June 15, 2026: We've paused the previously-announced changes to Claude Agent SDK usage. For now, nothing has changed: Claude Agent SDK, claude -p, and third-party app usage still draw from your subscription limits."

Corrections to the notes:
- The notes say Pro gets the October credits only implicitly. The text names "Max, Ultra, and Team" only. Pro is not listed.
- The June update explicitly names "third-party app usage" as drawing from subscription limits. That is a billing statement, not permission. It does show Anthropic expects third-party app traffic on plan logins to exist. It says nothing about ACP, IDEs or harnesses, and nothing about what is allowed.
- The body text of the original announcement is no longer on the page, so the "Agent SDK credit" amounts cannot be verified from this URL any more (they are only in secondary sources and the Zed blog).

## Claim 5. claude-agent-acp license, registry label, bundling

Verdict: PARTLY. Both labels exist. The discrepancy is explained by what is shipped, not by the source license. The release archives do bundle the Claude CLI.

Sources:
- LICENSE file: https://raw.githubusercontent.com/agentclientprotocol/claude-agent-acp/main/LICENSE is the Apache License 2.0, ending "Copyright 2025 Zed Industries, Inc. and contributors". GitHub API reports `apache-2.0`. History: "Add Apache2 license (#1)" 2025-08-28, "Update for ACP Org (#471)" 2026-03-26.
- package.json: `"license": "Apache-2.0"`, `"author": "Zed Industries"`, dependency `"@anthropic-ai/claude-agent-sdk": "0.3.322"`.
- Registry: https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json entry `claude-acp`: `"license": "proprietary"`, `"license_url": ".../claude-agent-acp/blob/main/LICENSE"` (which is Apache), `"authors": ["Anthropic", "Zed Industries", "JetBrains"]`, version 0.106.0, binary archives per platform.
- Registry history (github.com/agentclientprotocol/registry, `claude-acp/agent.json`): already `"proprietary"` at the first commit under that path (2026-02-17, "Claude Code -> Claude Agent", version 0.17.0, authors ["Anthropic"], npx-only distribution), and unchanged through 0.45.1, 0.85.1, 0.96.0 and 0.106.0.
- Agent SDK license: https://raw.githubusercontent.com/anthropics/claude-agent-sdk-typescript/main/LICENSE.md: "(c) Anthropic PBC. All rights reserved. Use is subject to Anthropic's Commercial Terms of Service."
- Claude Code license (npm `@anthropic-ai/claude-code@2.1.322` LICENSE.md): "(c) Anthropic PBC. All rights reserved. Use is subject to the Legal Agreements outlined here: https://code.claude.com/docs/en/legal-and-compliance." npm license field: "SEE LICENSE IN README.md".
- Adapter README, "Standalone executables": "Each holds two files: `claude-agent-acp`, and the Claude CLI (`claude`) it runs. Keep them in the same directory ... To use a Claude CLI you already have instead, set `CLAUDE_CODE_EXECUTABLE`." `scripts/build-binary.mjs`: "Builds an executable of the agent with `bun build --compile` and copies the target platform's Claude CLI next to it. Both files go in the release archive". Release v0.106.0 archives are 186 to 219 MB each.

Explanation of the discrepancy: the adapter source is Apache-2.0. Every way it is distributed pulls in Anthropic proprietary code: the npm package depends on the Agent SDK (All rights reserved, Commercial Terms), and the release archives contain both a compiled bundle of that SDK and the proprietary `claude` binary. The registry lists Anthropic as an author and labels the shipped agent "proprietary". The label predates binary distribution, so it reflects the SDK and Claude Code dependency, not only the bundled CLI. The registry's own `license_url` points at the Apache file, so the entry is internally inconsistent.

Redistribution terms of Claude Code: none granted. "All rights reserved" plus "Use is subject to the Legal Agreements". The Consumer Terms grant no copy or distribution right ("Other than the rights of access and use expressly granted in our Terms, our Terms do not grant you any right, title, or interest in or to our Services"). The legal page's "preinstalling ... requires agreeing to our Commercial Terms" is the only route named.

FocusIDE status (checked in this repo): `scripts/fetch-adapter.mjs` keeps only `claude-agent-acp` and deletes the bundled CLI ("The release ships its own Claude CLI next to the adapter. FocusIDE does not use it"), and `crates/core/src/acp.rs` sets `CLAUDE_CODE_EXECUTABLE` to the user's own `claude`. So FocusIDE does not redistribute the Claude CLI. It does redistribute the 113 MB adapter executable, which contains the compiled Agent SDK JavaScript. That part is not Apache-2.0. The notes' inference "Apache-2.0 is compatible with distributing it alongside a GPL-3.0 app" is incomplete: the shipped executable is a mix of Apache code and Anthropic code under Commercial Terms. GPL-3.0 does not apply to it as a separate program launched as a sidecar, but FocusIDE's installer redistributes Anthropic code, which needs the Commercial Terms route (claim 2).

Maintainers: top contributors are benbrandt (425 commits; Ben Brandt, Zed), nikita-ashihmin and Mark Tkachenko (jetbrains.com addresses), agu-z, ConradIrwin (Zed). No anthropic.com committers in the latest 15 commits. "Not an Anthropic product" holds for the source repo, but the registry lists Anthropic as first author.

Missed detail: the adapter has a `--hide-claude-auth` flag (`src/hide-claude-auth.ts`): "this integration must never bill a claude.ai subscription. Under the flag the agent hides the claude.ai login method from `initialize`, and refuses with `authRequired` every session and every turn that a subscription would pay for." Without the flag it advertises an ACP auth method "Claude Subscription" (`src/auth-methods.ts`, args `--cli auth login --claudeai`). A client that shows that button is arguably "offering Claude.ai login" in its own UI, even though the flow completes in Anthropic's CLI. FocusIDE does not currently surface `authMethods` (it only parses them in `crates/core/src/acp.rs`).

## Claim 6. Hooks and plugins

Verdict: CONFIRMED. One detail is wider than stated.

URLs: https://code.claude.com/docs/en/hooks , https://code.claude.com/docs/en/plugins-reference

- Stop: "In addition to the common input fields, Stop hooks receive `stop_hook_active`, `last_assistant_message`, `background_tasks`, and `session_crons`." and "The `last_assistant_message` field contains the text content of Claude's final response, so hooks can access it without parsing the transcript file."
- UserPromptSubmit: "UserPromptSubmit hooks receive the `prompt` field containing the submitted text." New detail: in sessions that mark pasted text, expanded paste "sits between a `<pasted_content id=\"...\">` line and a `</pasted_content id=\"...\">` line, so account for those lines if your hook parses the prompt."
- Plugin hooks: "Define plugin hooks in `hooks/hooks.json` with an optional top-level `description` field. When a plugin is enabled, its hooks merge with your user and project hooks." The file must wrap the event map in a top-level `"hooks"` key: "A file that contains only the event map, without that wrapper, fails to load."
- Reserved names (wider than the notes): errors for names that start with `claude-`, `anthropic-`, `anthropics-`, or `cc-plugin-`; names equal to `claude`, `anthropic`, `anthropics`, `claude-code`, `claude-mods`; names that put `official` beside `claude` or `anthropic`. Warning for `claude`, `anthropic` or `anthropics` as a whole word elsewhere (for example `mcp-for-claude`). "Claude Code still installs and loads a plugin whose name they refuse." `focuside` is clean; `focuside-for-claude` would draw a warning.

## Claim 7. Enforcement events

Verdict: PARTLY. Dates mostly hold, with corrections. The statement "no enforcement targeted tools that run the real `claude` binary" is CONTRADICTED by reports about OpenClaw's Claude CLI backend.

- Jan 9, 2026 server-side block: secondary only. The Register (https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access) does not give Jan 9; it quotes Thariq Shihipar's January thread: "Third-party harnesses using Claude subscriptions create problems for users and are prohibited by our Terms of Service," and that Anthropic had acted against tools "spoofing the Claude Code harness". The Jan 9 date comes from VentureBeat, piunikaweb and blogs. No Anthropic primary source. UNVERIFIABLE from primary sources.
- OpenCode: two separate events, both primary on GitHub.
  - Commit 973715f3 (https://github.com/anomalyco/opencode/commit/973715f3da1839ef2eba62d4140fe7441d539411), 2026-02-19, message "anthropic legal requests". It removed the `opencode-anthropic-auth` builtin plugin code and the 166-line `anthropic-20250930.txt` prompt file.
  - PR #18186 (https://github.com/anomalyco/opencode/pull/18186), merged 2026-03-19, title "anthropic legal requests", body: "Remove anthropic references per legal requests: Remove anthropic-20250930.txt prompt file; Remove anthropic from provider hints; Remove opencode-anthropic-auth builtin plugin; Remove anthropic from provider enum".
  - The notes date the legal requests to March 19 only. The first "legal requests" commit was February 19. Both targeted token reuse plus a Claude-Code-imitating system prompt, not the real binary.
- Apr 4, 2026: InfoWorld (https://www.infoworld.com/article/4154435/...) quotes the Anthropic email: "Starting April 4, third-party harnesses like OpenClaw connected to your Claude account will draw from extra usage instead of from your subscription." Cherny: "our subscriptions weren't built for the usage patterns of these third-party tools." It is a billing change (extra usage), not a block. "Third-party harness" is not defined.
- Missed: OpenClaw has a "Claude CLI" backend that runs the user's installed `claude -p` with their own login. OpenClaw docs (npm `openclaw@2026.6.9`, docs/providers/anthropic.md): "OpenClaw's Claude CLI backend runs the installed Claude Code CLI in non-interactive print mode." Hacker News story "Anthropic says OpenClaw-style Claude CLI usage is allowed again" (https://news.ycombinator.com/item?id=47844269, around April 20-21, 2026), top comment by Peter Steinberger (OpenClaw): "Boris from Claude Code said publicly on Twitter that CLI-style usage is allowed." and "in practice, Anthropic still blocks parts of our system prompt" and "They since seemed to changed their classifier as people hack around it". A comment quotes Boris on X: "This is not intentional, likely an overactive abuse classifier." (secondary; the X post itself was not retrieved). Takeaway: Anthropic runs a server-side classifier that looked at request content (system prompt) and hit traffic that went through the real `claude` binary. Staff then said CLI use is allowed, informally and not in docs.

## Claim 8. Zed and JetBrains guidance

Verdict: PARTLY.

- JetBrains Air (https://www.jetbrains.com/help/air/air-in-ides-authorization.html), current: "A Claude subscription cannot be used to authorize Claude Agent in the chat surface." "To use Claude Agent, select JetBrains AI or configure Anthropic API-key authentication with API billing." "To use your Claude subscription, run Claude Code in the terminal surface," "which Air in IDEs opens as a full-screen editor tab." CONFIRMED. The Aug 2026 Air blog about Claude subscriptions still returns HTTP 403; UNVERIFIABLE.
- Zed: the blog (https://zed.dev/blog/anthropic-subscription-changes, May 14, 2026, updated June 16, 2026) says "When the official `claude` CLI runs in the terminal, it uses your subscription's limits, not the new credit." That advice was about the paused billing split. The June update says ACP usage and the Agent SDK "continue to work with Claude subscriptions exactly as they did before." Zed's current docs (https://zed.dev/docs/ai/external-agents) do not tell subscription users to use the terminal: "Claude Agent owns its own authentication and billing." "To choose your billing method, open a Claude Agent thread, run `/login`," then "authenticate with an API key or with Claude Code where supported." So Zed currently lets plan users log in inside its ACP thread. "Zed guidance is to run `claude` in the terminal" is outdated.

---

## Risks for pane mode that the research missed

Pane mode = FocusIDE runs the user's own unmodified `claude` TUI in a PTY, plus a user-installed Claude Code plugin with hooks.

1. Content-based classifier. The OpenClaw case shows Anthropic classifies plan traffic by content, including when it comes through the real binary, and moves "third-party harness" traffic to extra usage or blocks it. Anything FocusIDE injects into the model context raises this risk: `--append-system-prompt`, `--system-prompt`, plugin skills, and UserPromptSubmit `additionalContext`. Keep injected text short, factual and specific to the user's project. Never inject a harness-style system prompt. Prefer hooks that only observe (write files) over hooks that add context.
2. Automated driving. Consumer Terms (https://www.anthropic.com/legal/consumer-terms, effective Oct 8, 2025), Section 3 bars access "through automated or non-human means, whether through a bot, script, or otherwise" except via an API key "or where we otherwise explicitly permit it". Pane mode is safe only while a human types. Writing prompts into the PTY from FocusIDE (queued tasks, "retry", auto-continue, scripted slash commands) is the step that turns it into scripted access on a consumer plan. Stop hooks that return `decision: "block"` to keep Claude working are also automation; the docs cap them at 8 in a row.
3. Restricting login methods. If pane mode passes env vars that force a login method (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `forceLoginMethod` in settings, `--bare`), it removes or restricts methods "built into" the binary, which the products clause forbids. Inherit the user's environment unchanged.
4. Naming and look. A tab or pane titled "Claude Code" is a feature name using the mark. FocusIDE chrome around the TUI that copies Claude Code ASCII art or colors would "mimic Claude Code". Use "Claude" or "Agent", and plain text such as "runs Claude Code".
5. Plugin install path. A plugin that FocusIDE writes into `~/.claude/settings.json` (`enabledPlugins`) or into a marketplace without the user doing it is FocusIDE changing the user's Claude Code config. Have the user run `/plugin install` or confirm a visible prompt. Managed settings can block it anyway: `allowManagedHooksOnly` restricts which hooks run, and `disableAllHooks` turns them all off. Pane mode features that depend on hooks must degrade when hooks do not fire.
6. `--bare` is the planned default for `-p`. Headless docs: "`--bare` is the recommended mode for scripted and SDK calls, and will become the default for `-p` in a future release." and "In bare mode, Claude Code never reads OAuth credentials or the system keychain." and "Hooks still don't run in bare mode when you pass them ... in the `hooks/hooks.json` ... of a plugin you load with a flag." This does not affect the interactive TUI, but any fallback to `claude -p` (for titles, summaries, checks) will lose plan login and plugin hooks when the default changes. It also affects the SDK path used by ACP mode.
7. Trust prompts in `-p`. "A `-p` session shows no workspace trust dialog and no per-server approval prompt" and runs project hooks and `.mcp.json` servers "even in a folder you've never trusted". If FocusIDE ever runs `claude -p` in a user's repo in the background, it runs that repo's hooks without consent. The TUI shows the trust dialog, so pane mode itself is fine.
8. Telemetry argument. Thariq's reason was that harnesses send traffic "without any of the usual telemetry that the Claude Code harness provides". Pane mode keeps the real binary and its telemetry, which is the strongest argument in its favour. Do not set variables that disable Claude Code telemetry or nonessential traffic on the user's behalf.
9. Policy can change without notice. Legal page: "may do so without prior notice". The support article was rewritten twice in five months. The only written permission for pane mode is the one-sentence carve-out. There is no approval process other than "contact sales". Ship pane mode with a visible note that billing follows the user's own Claude Code login and Anthropic's terms.

## Sources fetched

- https://code.claude.com/docs/en/legal-and-compliance
- https://code.claude.com/docs/en/agent-sdk/overview
- https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan
- https://code.claude.com/docs/en/hooks.md
- https://code.claude.com/docs/en/plugins-reference.md
- https://code.claude.com/docs/en/headless.md
- https://code.claude.com/docs/en/authentication.md
- https://www.anthropic.com/legal/consumer-terms
- https://github.com/agentclientprotocol/claude-agent-acp (LICENSE, package.json, README.md, scripts/build-binary.mjs, src/auth-methods.ts, src/hide-claude-auth.ts, releases/latest)
- https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json and https://github.com/agentclientprotocol/registry history of claude-acp/agent.json
- https://raw.githubusercontent.com/anthropics/claude-agent-sdk-typescript/main/LICENSE.md
- https://unpkg.com/@anthropic-ai/claude-code@2.1.322/LICENSE.md
- https://github.com/anomalyco/opencode/commit/973715f3da1839ef2eba62d4140fe7441d539411 and https://github.com/anomalyco/opencode/pull/18186
- https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access
- https://www.infoworld.com/article/4154435/anthropic-cuts-openclaw-access-from-claude-subscriptions-offers-credits-to-ease-transition.html
- https://news.ycombinator.com/item?id=47844269 (secondary for staff quotes)
- https://cdn.jsdelivr.net/npm/openclaw@2026.6.9/docs/providers/anthropic.md
- https://zed.dev/blog/anthropic-subscription-changes and https://zed.dev/docs/ai/external-agents
- https://www.jetbrains.com/help/air/air-in-ides-authorization.html
