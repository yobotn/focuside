# Challenge 1: adversarial check of non-Anthropic agent claims (as of 2026-11-12)

Method: each claim from openai_codex.md, gemini_cli.md, copilot_and_others.md and open_source_and_acp.md was tested against live primary pages fetched on 2026-11-12. The goal was to disprove, not confirm. The fetch tool returns condensed text, so quotes are short and should be spot-checked before publishing. openai.com/policies and help.openai.com still return HTTP 403 to the fetch tool.

Verdict key: CONFIRMED, CONTRADICTED, PARTLY, UNVERIFIABLE.

## Summary table

| # | Claim | Verdict |
| --- | --- | --- |
| 1a | OpenAI SIWC program, eligibility quote | CONFIRMED (quote exact). Launch date "around Sept 2026" is from secondary press only |
| 1b | SIWC integration requires the app to hold OAuth tokens | CONFIRMED |
| 1c | SIWC addresses spawning the user's own Codex with Codex's own login | UNVERIFIABLE (not addressed anywhere) |
| 2a | codex-acp moved to agentclientprotocol/codex-acp, Apache-2.0 | CONFIRMED |
| 2b | Codex hooks with UserPromptSubmit/Stop and hash-based trust | CONFIRMED |
| 3a | May 19 2026 Gemini CLI to Antigravity CLI announcement | CONFIRMED |
| 3b | From June 18 2026 Login with Google no longer works in Gemini CLI for consumer accounts | CONFIRMED |
| 3c | Gemini CLI still works with API keys and Code Assist Standard/Enterprise | PARTLY (Google says "paid" API keys; Standard/Enterprise have a pending separate deprecation schedule) |
| 3d | `agy acp` exists and is intended for third-party ACP clients | CONFIRMED, with a model-access limit for unlisted clients |
| 3e | Google bans harvesting/piggybacking Gemini CLI OAuth | CONFIRMED, but Antigravity terms and FAQ are broader than "harvesting" |
| 3f | Suspensions for OpenCode etc. that spawn the official binary (vs reuse tokens) | PARTLY: documented suspensions are tied to token reuse; one unclear Nov 2026 report mentions Zed ACP; no confirmed case for spawning the official binary |
| 4a | `copilot --acp` public preview since Jan 2026 | CONFIRMED (2026-01-28), still preview |
| 4b | Copilot SDK GA June 2026 for your own apps | CONFIRMED (2026-06-02) |
| 4c | Formal GitHub partnership with OpenCode, Jan 2026 | CONFIRMED (2026-01-16) |
| 4d | No GitHub terms clause forbidding third-party clients | CONFIRMED (none found) |
| 5 | Gemini CLI hooks BeforeAgent/AfterAgent | CONFIRMED |
| 6a | OpenCode, Goose, Cline, Qwen Code, Kimi, Mistral Vibe have native ACP | CONFIRMED (first-party ACP entry points; Mistral uses a separate first-party `vibe-acp` binary) |
| 6b | Registry at cdn.agentclientprotocol.com/registry/v1/latest/registry.json | CONFIRMED (HTTP 200, version 1.0.0, 45 agents) |

## 1. OpenAI "Sign in with ChatGPT" (SIWC)

### 1a. Program and eligibility: CONFIRMED (date PARTLY)
- Quote: "At launch, ChatGPT plan usage is available to open-source projects, personal projects that run locally, and selected private apps." Also: "Identity-only integration is currently offered to a select group of commercial partners." Paid or hosted apps must "join the waitlist" and "request access before offering it to users." The page also says: "Review OpenAI's terms and policies before distributing your integration." URL: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt
- Date: no date appears on the OpenAI pages. The September 29, 2026 launch date comes only from secondary coverage (The New Stack, via search). Correction: say "launched at DevDay, reported Sept 29 2026 (secondary)", not a primary-sourced date.

### 1b. What the integration requires: CONFIRMED, the app holds OAuth tokens
- Registration: "The OAuth flow for OSS clients registers a user-defined agent during Sign in with ChatGPT." "each issued `client_id` is bound to the authenticated user and the workspace selected during registration." The app must persist an `ext_agent_host_id` before first sign-in. After the code exchange the app must "store the user's basic account info and credentials." URL: https://developers.openai.com/siwc/token-sharing-open-source
- Codex app-server path: "Set `ACCESS_TOKEN` in the app-server child process's environment to that value." "Your app should read its own token file and supply the access token to the child process." "No separate Codex sign-in is required." URL: https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- The Cookbook DevKit keeps tokens in the app's main process; "a small bridge lets the interface call these functions without receiving tokens." The app still stores and refreshes them. URL: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt
- Consequence for FocusIDE: SIWC conflicts with the rule "FocusIDE never reads, stores or proxies agent credentials". This holds.

### 1c. Spawning the user's own Codex / app-server / codex-acp with Codex's own login: UNVERIFIABLE (not addressed)
- None of the three SIWC pages mention Codex CLI's own login, `auth.json`, or apps that launch the user's own Codex. The app-server SIWC page shows `codex app-server` but "does not say which binary is used or whether Codex's stored credentials can be reused" (fetch summary).
- Positive signals (not a permission): the app-server docs say it is for a "deep integration inside your own product", and enterprise integrators should "contact OpenAI to get it added to a known clients list" and set `clientInfo.name` "to identify your client for the OpenAI Compliance Logs Platform". URL: https://learn.chatgpt.com/docs/app-server
- Codex auth docs: "Access tokens are intended for trusted scripts, schedulers, and private CI runners." "API keys are still the recommended default for automation." "treat `~/.codex/auth.json` like a password". No third-party-client clause. URL: https://learn.chatgpt.com/docs/auth
- OpenAI Terms of Use (account-sharing and programmatic-extraction clauses): still HTTP 403. UNVERIFIABLE from the live page.
- Correction to openai_codex.md: the statement "Nothing found forbids a local GUI spawning the user's own Codex" stays accurate, but the report should say OpenAI has neither allowed nor forbidden it in writing. The only written sanctioned path for a third-party app using a ChatGPT plan is SIWC.

## 2. codex-acp and Codex hooks

### 2a. Repo move: CONFIRMED
- zed-industries/codex-acp: archived "Jul 22, 2026", read-only; development moved to agentclientprotocol/codex-acp, built on the Codex App Server; new installs use `@agentclientprotocol/codex-acp`. License Apache-2.0. URL: https://github.com/zed-industries/codex-acp
- agentclientprotocol/codex-acp: "ACP server implementation that exposes Codex CLI functionality for smoother client and IDE integration." Contributions "licensed under the Apache 2.0 License". Auth: ChatGPT login (hidden with `NO_BROWSER=1`), `CODEX_API_KEY` / `OPENAI_API_KEY`, opt-in gateway. The page does not say OpenAI maintains it. URL: https://github.com/agentclientprotocol/codex-acp
- Registry entry: `codex-acp` 2.6.0, Apache-2.0, npx `@agentclientprotocol/codex-acp@2.6.0`.

### 2b. Hooks: CONFIRMED
- "Hooks are enabled by default." Disable with `[features] hooks = false`. UserPromptSubmit and Stop are turn-scoped and carry `turn_id` ("Active Codex turn id", a Codex-specific extension). Trust: "Codex records trust against the hook's current hash." New or changed non-managed hooks are skipped until trusted; managed hooks are "trusted by policy." URL: https://learn.chatgpt.com/docs/hooks

## 3. Google: Gemini CLI, Antigravity CLI

### 3a. May 19 2026 announcement: CONFIRMED
- Dated "MAY 19, 2026". URL: https://developers.googleblog.com/en/an-important-update-transitioning-gemini-cli-to-antigravity-cli/

### 3b. June 18 2026 Login with Google cutoff: CONFIRMED
- "Starting June 18, 2026, Gemini Code Assist IDE extensions stopped serving requests" for the individuals, Google AI Pro and Ultra tiers. "This also applies to usage of Gemini CLI." "you can no longer use the Login with Google option to access the IDE extensions or Gemini CLI." URL: https://developers.google.com/gemini-code-assist/resources/privacy-notice-gemini-code-assist-individuals
- FAQ banner: "Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." URL: https://geminicli.com/docs/resources/faq/

### 3c. What still works: PARTLY
- Blog: "Gemini CLI will remain accessible via paid Gemini and Gemini Enterprise Agent Platform API keys." Standard/Enterprise: "your access remains unchanged." URL: Google Developers Blog above.
- Code Assist page: "Gemini Code Assist Standard and Enterprise subscriptions are subject to a separate deprecation schedule." No dates given.
- Correction: write "paid Gemini API keys and Gemini Enterprise Agent Platform keys", not "API keys". Free AI Studio keys are not named. Add that Standard/Enterprise access is "unchanged" for now but has its own future deprecation schedule.

### 3d. `agy acp` for third-party ACP clients: CONFIRMED, with limits
- "The `agy acp` command starts an Agent Client Protocol (ACP) server." Editors launch it over stdio. "`agy acp` reuses the cached credentials." URL: https://antigravity.google/docs/cli/acp
- "Any ACP client can launch Antigravity CLI." "The server speaks ACP protocol version 1 over `stdio`". Auth methods advertised: `oauth-personal`, `oauth-business`, `gemini-api-key`, `agent-platform`, `terminal-login` (terminal login only for clients that "support ACP terminal authentication"). URL: https://antigravity.google/docs/ide/extensions/acp/
- Limit: Gemini models work in every client, but third-party models are offered only in "JetBrains IDEs (including JetBrains Air), Android Studio, Android Performance Analyzer, Zed, and Xcode." FocusIDE would get Gemini models only.
- New detail: the ACP registry entry `antigravity-acp` 1.4.5 (proprietary) downloads a separate binary `agy_acp_server.par` from dl.google.com, not the `agy` CLI. FocusIDE can use either; the docs path is `agy acp`.

### 3e. Terms ban OAuth harvesting/piggybacking: CONFIRMED, and broader than claimed
- Gemini CLI FAQ: "Using third-party software, tools, or services to harvest or piggyback on Gemini CLI's OAuth authentication" is prohibited; "the supported and secure method is to use a Vertex AI or Google AI Studio API key." URL: https://geminicli.com/docs/resources/faq/
- Gemini CLI repo: "Directly accessing the services powering Gemini CLI (for example, the Gemini Code Assist service)" "using third-party software, tools, or services (for example, using OpenClaw with Gemini CLI OAuth)" "is a violation of applicable terms and policies." "Such actions may be grounds for suspension or termination of your account." URL: https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md
- MISSED in gemini_cli.md (now verified from the primary page, which the earlier note took from blogs): "Google Antigravity Additional Terms of Service", clause 6: using third-party software, tools, or services to access the Service "(e.g. using OpenClaw with Antigravity OAuth) is a breach of this Agreement"; "Such actions may be grounds for suspension or termination of your Antigravity and/or Gemini CLI accounts." No effective date shown. URL: https://antigravity.google/terms
- MISSED: Antigravity FAQ: "Why can't I use third-party software (such as Claude Code, OpenClaw, or OpenCode) with my Antigravity login?" Answer: "Using third-party software, tools, or services to access Antigravity is a violation of our Terms of Service" and "Such actions can result in suspension or termination of your account." URL: https://antigravity.google/docs/faq.md
- Correction: the Antigravity wording is not limited to "harvesting" tokens. Read literally, it covers any third-party software that reaches Antigravity with an Antigravity login. The official ACP docs ("Any ACP client can launch Antigravity CLI") are the counterweight. FocusIDE should only reach Antigravity through `agy acp` (the documented path) or the `agy` TUI in a terminal pane, never through `agy -p` orchestration.
- Forum replies conflict (no staff badge visible on either page):
  - Rebbanpalli_Naveen, 2026-09-16: "Yes, your described pattern is compliant" for local PTY/subprocess wrapping of `agy` for an editor bridge, and the ACP server is the "officially supported path". URL: https://discuss.ai.google.dev/t/clarification-request-is-local-ipc-pty-wrapping-of-agy-cli-for-open-standard-editor-bridges-e-g-acp-tos-compliant/175905
  - Ambati_Rajendra, 2026-09-25: wrapping `agy -p` in a local MCP server for third-party agents is "not permitted"; Google AI Pro quota is "strictly for direct use within official Antigravity tools". URL: https://discuss.ai.google.dev/t/is-using-the-official-agy-cli-through-a-local-mcp-server-with-third-party-ai-agents-permitted/184829

### 3f. Suspensions for clients that spawn the official binary: PARTLY
- Documented suspensions name OpenClaw and OpenCode, and the OpenCode cases are tied to an OAuth plugin (token reuse). Forum thread titles: "Account banned after using OpenCode OAuth plugin" (https://discuss.ai.google.dev/t/account-banned-after-using-opencode-oauth-plugin-appeal-request/130898), "[Appeal] Account Suspended (403) - OpenCode use admitted and removed" (https://discuss.ai.google.dev/t/appeal-account-suspended-403-opencode-use-admitted-and-removed/137252). Search summaries say one user was using "OpenCode with the Antigravity plugin, not the official application".
- Enterprise DNA (2026-09-03) says Google suspends accounts that "connect third-party agent tools, including OpenClaw and OpenCode, to Google's OAuth and backend systems", cites no clause, and does not say which mechanism. URL: https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-09-03-google-suspends-paying-antigravity-subscribers-for-using-thi
- Counter-evidence to watch: thread titled "Appeal for Antigravity Account Suspension for using Official Zed Gemini ACP" (2026-11-08). The body only says "I may have unintentionally triggered a restriction while experimenting with a third-party development tool"; no cause confirmed; the reply tells the user to remove "third-party wrappers" and revoke third-party OAuth access. URL: https://discuss.ai.google.dev/t/appeal-for-antigravity-account-suspension-for-using-official-zed-gemini-acp/198956
- Several threads report 403 suspensions while using the official `agy` CLI with no stated cause (for example https://discuss.ai.google.dev/t/support-request-antigravity-cli-1-1-4-403-permission-denied-tos-violation-false-positive/175558).
- Correction: "no confirmed suspension for spawning the official binary" is accurate, but false-positive suspensions of official-binary users exist, and Google's detection is opaque. FocusIDE copy should not promise that the ACP path is safe from suspension.

## 4. GitHub Copilot

- 4a CONFIRMED: changelog dated "January 28, 2026", "ACP support in Copilot CLI is now in public preview", `copilot --acp`; use cases include "Custom frontends tailored to specific developer workflows". URL: https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/ . The reference page still says "in public preview and subject to change." URL: https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server
- 4b CONFIRMED: "June 2, 2026", "The GitHub Copilot SDK is now generally available", embeds the engine into "your own applications, services, and developer tools"; available to all Copilot subscribers including Copilot Free, and to non-Copilot users via BYOK. URL: https://github.blog/changelog/2026-06-02-copilot-sdk-is-now-generally-available/
- 4c CONFIRMED: "January 16, 2026", "GitHub Copilot now fully supports authentication with OpenCode through a formal partnership." Plans: "Pro, Pro+, Business, or Enterprise". URL: https://github.blog/changelog/2026-01-16-github-copilot-now-supports-opencode/
- 4d CONFIRMED (absence): Acceptable Use Policies forbid "excessive automated bulk activity" and placing "undue burden on our servers through automated means", but say nothing about Copilot or third-party clients. URL: https://docs.github.com/en/site-policy/acceptable-use-policies/github-acceptable-use-policies . Generative AI Services Terms (Version March 2026): third-party products used "at your own risk"; no clause on third-party clients, automation, credential sharing or rate limits. URL: https://github.com/customer-terms/github-generative-ai-services-terms

## 5. Gemini CLI hooks: CONFIRMED
- BeforeAgent: "After user submits prompt, before planning". AfterAgent: "When agent loop ends". The page does not say whether hooks are on by default. URL: https://geminicli.com/docs/hooks/
- Note: Antigravity CLI keeps "Agent Skills, Hooks, Subagents, and Extensions (now as Antigravity plugins)" per the May 19 blog; Antigravity hook event names were not verified.

## 6. Native ACP agents and registry: CONFIRMED

Registry fetched with curl, HTTP 200, `version` 1.0.0, 45 agents. URL: https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json

| Agent | Registry id, version, license | Launch (all first-party, no third-party adapter) |
| --- | --- | --- |
| OpenCode | opencode 2.2.7, MIT (also opencode-v2) | `opencode acp` |
| Goose | goose 1.59.0, Apache-2.0 | `goose acp` |
| Cline | cline 3.0.74, Apache-2.0 | `cline --acp` |
| Qwen Code | qwen-code 0.27.2, Apache-2.0 | `qwen --acp --experimental-skills` |
| Kimi | kimi-code 2.6.0, MIT (`acp`); kimi "Kimi CLI (Legacy)" 1.52.0 | `acp` subcommand |
| Mistral Vibe | mistral-vibe 3.1.1, Apache-2.0 | separate first-party binary `vibe-acp` from github.com/mistralai |

Also in the registry: gemini 0.68.0 (`--acp`), github-copilot-cli 1.0.109 (proprietary, `--acp`), cursor 2026.11.11 (proprietary, `acp`), kiro 3.0.0 (proprietary), antigravity-acp 1.4.5 (proprietary), devin, junie, grok-build, factory-droid, auggie. Note: `fast-agent` ships with env `FAST_AGENT_MODEL=codexplan`, a third-party agent that uses a ChatGPT plan by default.

## Missed items

### Cursor
- ACP docs: "ACP is intended for building custom clients and integrations." "Build or use third-party integrations for your preferred development environment." URL: https://cursor.com/docs/cli/acp
- Terms of Service, "Last updated September 3, 2026": no clause on third-party clients or calling the CLI from other apps. Relevant clauses: no "harvest, scrape, or extract data from the Service" (1.5(viii)), no reverse engineering (1.5(i)), account holder responsible for "all activities that occur under your account" (3). Training: "ANYSPHERE WILL NOT USE CONTENT TO TRAIN, OR ALLOW ANY THIRD PARTY TO TRAIN, ANY AI MODELS" unless you agreed. URL: https://cursor.com/terms-of-service
- Verdict: no Cursor restriction on a third-party ACP client found.

### Kiro
- "Any editor supporting ACP can integrate Kiro by spawning `kiro-cli acp` and communicating via JSON-RPC over stdio." "`kiro-cli acp` needs a signed-in Kiro account." URL: https://kiro.dev/docs/cli/acp/
- Data: for Free Tier and individual subscribers, "Subject to your opt out, content may be used for service improvement." "We do not use content from Kiro enterprise users for service improvement." Free Tier: "Kiro may also store inputs for up to 60 days for additional abuse detection", not affected by opt-out. URL: https://kiro.dev/docs/privacy-and-security/data-protection/
- AWS Service Terms clauses on third-party clients not checked.

### Data-training defaults by auth mode (new or corrected)
- GitHub Copilot: from April 24, 2026, Copilot Free, Pro and Pro+ interaction data is used for model training "unless they opt out" (Copilot settings, Privacy section). "Copilot Business and Copilot Enterprise users are not affected." The post does not say whether CLI or third-party clients are treated differently, so assume they are covered. URL: https://github.blog/news-insights/company-news/updates-to-github-copilot-interaction-data-usage-policy/ . Generative AI Services Terms (Business/Enterprise): "GitHub will not use Inputs or Outputs to train generative AI models." copilot_and_others.md did not record the individual-plan default; add it.
- Antigravity (consumer): Additional Terms clause 5 says Google uses Interactions to improve products and machine learning technologies, and employees and contractors may review them; users can change the preference in settings. URL: https://antigravity.google/terms . The FAQ says "You can opt out of data collection at any point from the Settings panel." gemini_cli.md listed this as a gap; it is now verified.
- Gemini API: unpaid keys used for improvement with human review; paid keys not (already in gemini_cli.md, not re-fetched).
- Codex with a ChatGPT plan: per a help center search snippet (page 403), "Your ChatGPT training data controls apply to content processed through Codex". So Plus/Pro users with "Improve the model for everyone" on may have Codex content used for training; API keys are not used for training by default. UNVERIFIABLE from the live page.
- Cursor: no training unless the user agreed (see above).

### Vendors that forbid embedding their TUI in another app
- None found. No Codex, Gemini, Antigravity, Copilot, Cursor or Kiro document addresses embedding the vendor's own TUI in a terminal pane of another app. The Antigravity FAQ's "third-party software ... to access Antigravity" wording is the closest text; running the official `agy` TUI in a terminal emulator is the user running the official tool, but Google has not said so in writing.
- Copilot CLI license (from the earlier note, not re-fetched) allows redistribution only "in unmodified form" as part of an app with "material functionality". Relevant only if FocusIDE bundles the binary.

## Corrections to apply to the earlier notes
1. openai_codex.md: mark the SIWC launch date as secondary-sourced. State that OpenAI has written nothing, for or against, about third-party apps spawning Codex with Codex's own login.
2. gemini_cli.md: change "API keys" to "paid Gemini API keys and Gemini Enterprise Agent Platform keys". Replace the secondary clause 6 citation with the primary https://antigravity.google/terms and add the Antigravity FAQ entry that names Claude Code, OpenClaw and OpenCode. Add the conflicting forum reply (MCP wrapping "not permitted"). Add that unlisted ACP clients get Gemini models only. Note the Nov 8 2026 appeal that mentions Zed ACP, and official-binary false positives.
3. copilot_and_others.md: add the April 24 2026 training default for Copilot Free/Pro/Pro+; add the Cursor ToS date and its training clause; add Kiro data-use defaults.
4. open_source_and_acp.md: note that Mistral Vibe's ACP entry is a separate `vibe-acp` binary and that Antigravity's registry entry is a separate `agy_acp_server` binary.
