# Gemini CLI: terms and technical facts for a third-party desktop app (as of 2026-11-12)

Headline fact for the report writer: Gemini CLI is no longer the main Google consumer coding CLI. On May 19, 2026 Google announced that Gemini CLI is being replaced by Antigravity CLI (`agy`). From June 18, 2026, "Login with Google" in Gemini CLI stopped working for the free Code Assist for individuals, Google AI Pro and Google AI Ultra tiers. Gemini CLI still works with paid API keys and with Code Assist Standard/Enterprise licenses (which have "a separate deprecation schedule"). Antigravity CLI has an official ACP server (`agy acp`). Any FocusIDE integration plan should treat Antigravity CLI as the consumer path and Gemini CLI as an API key or enterprise path.

## Q1. Auth modes and what the terms say about third-party clients and programmatic use

### Takeaway
Google forbids using Gemini CLI (or Antigravity) OAuth credentials from third-party software to reach Google's backend, and has enforced this with account suspensions (including paid Ultra subscribers). Running the official, unmodified CLI binary as a local child process is not addressed by an official Google statement for Gemini CLI. For Antigravity CLI, Google documents ACP clients (Zed, JetBrains, Xcode, Android Studio, "other ACP clients") as a supported path. The supported route Google names for "a third-party coding agent" is an API key (AI Studio or Vertex AI).

### Cited Findings

Status of each auth mode (Nov 2026):
- Login with Google, free Code Assist for individuals / Google AI Pro / Ultra: discontinued for Gemini CLI. Quote: "Starting June 18, 2026, Gemini Code Assist IDE extensions stopped serving requests for the Gemini Code Assist for individuals, Google AI Pro, and Google AI Ultra tiers. This also applies to usage of Gemini CLI." and "As part of the deprecation, you can no longer use the Login with Google option to access the IDE extensions or Gemini CLI." Users "can migrate to the Antigravity family of products." [Gemini Code Assist for individuals page](https://developers.google.com/gemini-code-assist/resources/privacy-notice-gemini-code-assist-individuals)
- Same page: "Gemini Code Assist Standard and Enterprise subscriptions are subject to a separate deprecation schedule." [Code Assist individuals page](https://developers.google.com/gemini-code-assist/resources/privacy-notice-gemini-code-assist-individuals)
- Google blog, May 19, 2026: "On June 18, 2026, Gemini CLI and Gemini Code Assist IDE extensions will stop serving requests for Google AI Pro and Ultra" (and free individuals). Code Assist Standard/Enterprise: "your access remains unchanged." "Gemini CLI will remain accessible via paid Gemini and Gemini Enterprise Agent Platform API keys." Vertex AI is not named explicitly. [Google Developers Blog](https://developers.googleblog.com/en/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- Gemini CLI FAQ: "Unpaid tier and Google One users: Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." Also: "Gemini CLI has not been optimized for Gemini 4 Argon." "To use Gemini 4 Argon, please use Antigravity." [Gemini CLI FAQ](https://geminicli.com/docs/resources/faq/)
- Earlier (March 18, 2026, Ryan Salva announcement): effective March 25, 2026, free tier limited to Flash models, Pro models only for paid subscriptions; traffic priority "based on license type and account standing"; Google is "implementing more robust detection for policy-violating use cases" with the example "using Gemini CLI oAuth with third-party software"; recommends a paid AI Studio or Vertex AI key. [GitHub discussion #22970](https://github.com/google-gemini/gemini-cli/discussions/22970)

Gemini CLI repo terms doc (docs/resources/tos-privacy.md, no date shown):
- "Gemini CLI is an open-source tool that lets you interact with Google's powerful AI services" and "Gemini CLI software is licensed under the Apache 2.0 license." [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)
- "Directly accessing the services powering Gemini CLI ... using third-party software, tools, or services (for example, using OpenClaw with Gemini CLI OAuth) is a violation of applicable terms and policies." Violations may lead to suspension or termination. (Quote reassembled from fragments returned by the fetch tool; the ellipsis marks text not captured.) [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)
- Terms by auth method per that doc: Google account goes to Gemini Code Assist terms ([privacy notices](https://developers.google.com/gemini-code-assist/resources/privacy-notices)); Gemini Developer API key unpaid ([terms#unpaid-services](https://ai.google.dev/gemini-api/terms#unpaid-services)) and paid ([terms#paid-services](https://ai.google.dev/gemini-api/terms#paid-services)) with the Google Privacy Policy; Vertex AI key goes to [Google Cloud Service Terms](https://cloud.google.com/terms/service-terms/) and the [Cloud Privacy Notice](https://cloud.google.com/terms/cloud-privacy-notice). [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)
- Usage statistics: "Your Gemini CLI Usage Statistics are handled in accordance with Google's Privacy Policy." Opt-out via configuration docs. [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)

Gemini CLI FAQ on third-party tools:
- "Using third-party software, tools, or services to harvest or piggyback on Gemini CLI's OAuth authentication to access our backend services is a direct violation of our applicable terms and policies. Doing so bypasses our intended authentication and security structures, and such actions may be grounds for immediate suspension or termination of your account." [Gemini CLI FAQ](https://geminicli.com/docs/resources/faq/)
- "If you would like to use a third-party coding agent with Gemini, the supported and secure method is to use a Vertex AI or Google AI Studio API key." [Gemini CLI FAQ](https://geminicli.com/docs/resources/faq/)

Gemini API (AI Studio) terms, "Effective March 23, 2026" (ai.google.dev/gemini-api/terms now redirects here):
- "You must be 18 years of age or older to use the APIs." [Gemini API Additional Terms](https://aistudio.google.com/docs/terms)
- "You may use only Paid Services when making API Clients available to users in the European Economic Area," with the same for Switzerland and the UK. "You may only access the Services (or make API Clients available to users) within an available region." [Gemini API Additional Terms](https://aistudio.google.com/docs/terms)
- "You may not use the Services to develop models that compete with the Services"; no reverse engineering. No clause found on API key sharing. [Gemini API Additional Terms](https://aistudio.google.com/docs/terms)

Enforcement reports:
- Many forum threads report "403 ToS" suspensions of Antigravity and Gemini CLI / Code Assist access after using OpenClaw with Google OAuth, including Google AI Ultra subscribers. Examples: [forum: Ultra / OpenClaw](https://discuss.ai.google.dev/t/account-restricted-without-warning-google-ai-ultra-oauth-via-openclaw/122778/9), [forum: 403 appeal](https://discuss.ai.google.dev/t/appeal-submitted-403-tos-suspension-on-antigravity-gemini-cli-requesting-manual-review-ai-pro-subscriber/183563), [Enterprise DNA, Sept 2026](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-09-03-google-suspends-paying-antigravity-subscribers-for-using-thi), [GitHub discussion #20632](https://github.com/google-gemini/gemini-cli/discussions/20632) (notes Antigravity bans also blocked Gemini CLI and Code Assist).
- Secondary sources report Antigravity Additional Terms clause 6 says use of third-party software with Antigravity OAuth (example OpenClaw) is a breach and "may be grounds for suspension of Antigravity and/or Gemini CLI accounts". Not verified from Google's primary terms page. [zentor.ai](https://zentor.ai/blog/antigravity-account-suspended-third-party); [moclaw.ai](https://moclaw.ai/blog/antigravity-account-suspended-third-party)

Wrapping the official CLI (the FocusIDE pattern):
- GitHub issue #24011 (March 27, 2026) asked directly: "Does using Gemini CLI through a programmatic wrapper/adapter violate Google's or Gemini's Terms of Service?" No maintainer gave a ToS answer. A contributor replied "we don't have immediate plans to address this specific issue" and closed it (May 10, 2026). Community replies pointed to headless mode and ACP. [Issue #24011](https://github.com/google-gemini/gemini-cli/issues/24011)
- Google AI forum thread (July 23, 2026) asked whether local PTY/subprocess wrapping of `agy` for a single-user editor bridge is compliant. A reply on Sept 16, 2026 by a user not identified as Google staff said "Yes, your described pattern is compliant", conditional on not extracting, storing or forwarding login tokens, and said "Connecting directly to the official ACP server binary is the officially supported path". Treat as unofficial. [Google AI forum thread](https://discuss.ai.google.dev/t/clarification-request-is-local-ipc-pty-wrapping-of-agy-cli-for-open-standard-editor-bridges-e-g-acp-tos-compliant/175905)

### Inferences
- What Google targets is credential harvesting: third-party code that takes the CLI's OAuth token and calls Google's private backend directly. FocusIDE spawns the official binary and never touches credentials, which is a different pattern. The Gemini CLI issue thread and the (unofficial) forum reply both draw that line, but Google has issued no official statement saying that wrapping the Gemini CLI binary is allowed.
- For the ACP path, Google's own docs list ACP editors as supported clients for `agy acp`, including "other ACP clients". That is the strongest official signal that a third-party ACP host is fine.
- For Gemini CLI specifically in Nov 2026, the users left are API key users (AI Studio paid, Vertex AI / Gemini Enterprise Agent Platform) and Code Assist Standard/Enterprise users. For API key users the terms ask nothing extra of a client app. FocusIDE should only surface the 18+ rule and the EEA/UK/CH paid-only rule as user-facing notes.
- Free-tier Gemini API keys in EEA/UK/CH: the "make API Clients available to users" wording targets developers who build API clients. FocusIDE does not supply a key (the user brings their own to the CLI), so this most likely falls on the user, not FocusIDE. This is an interpretation, not legal advice.

### Gaps
- No primary-source text from the Antigravity Additional Terms was fetched. Clause 6 wording is from secondary blogs.
- Code Assist Standard/Enterprise deprecation schedule for Gemini CLI is not published in the sources found.
- Whether Vertex AI keys keep working with Gemini CLI after June 2026 is implied ("Gemini Enterprise Agent Platform API keys") but Vertex is not named.
- No official Google statement on embedding the Gemini CLI TUI in a terminal pane.

## Q2. ACP support in Gemini CLI (and Antigravity CLI), status, Zed

### Takeaway
Gemini CLI was the first ACP agent (Zed's reference integration, using `--experimental-acp`). Current Gemini CLI docs use `gemini --acp`. The successor, Antigravity CLI, ships an official ACP server, `agy acp`, documented for Zed, JetBrains, Android Studio, Xcode and other ACP clients, and listed in the ACP Agent Registry.

### Cited Findings
- Gemini CLI ACP docs: start with `gemini --acp`. Supported methods: `initialize` (client can register its MCP server), `authenticate`, `newSession`, `loadSession`, `prompt`, `cancel`, `setSessionMode` (approval level for tool calls), `unstable_setSessionModel`. The page links to the ACP Agent Registry for "specific IDEs like JetBrains and Zed". No experimental/stable label found. [Gemini CLI ACP mode](https://geminicli.com/docs/cli/acp-mode/)
- Zed's early integration used `--experimental-acp` (Zed commit "Tweaks to ACP for the Gemini PR (#34506)" changed `--acp` to `--experimental-acp`; later commit #35624 handles old Gemini CLI versions that lack the flag). [Zed commit #34506](https://git.secluded.site/zed/commit/fc24102491c3a644e53792c1a318b00bfdfd6d6b); [Zed commit #35624](https://git.secluded.site/zed/commit/a508a9536fb4a8b46e36ef74ef23a642f17daab6)
- Gemini CLI is listed in Zed's ACP agent directory. [Zed: Gemini CLI](https://zed.dev/acp/agent/gemini-cli)
- Antigravity CLI ACP docs: "`agy acp [flags]` starts an ACP server." The editor launches it as a subprocess over JSON-RPC on stdin/stdout. Auth: sign in once with interactive `agy` or `agy acp --login`; `agy acp` reuses cached credentials. Zed config: `agent_servers` entry with `"type": "custom"`, `"command": "agy"`, `"args": ["acp"]`. Names Zed, JetBrains IDEs, Android Studio, Xcode. [Antigravity CLI ACP](https://antigravity.google/docs/cli/acp.md); [Other editors (ACP)](https://antigravity.google/docs/ide/extensions/acp/)
- Antigravity ACP is in Zed's registry. [Zed: antigravity-acp](https://zed.dev/acp/agent/antigravity-acp)
- Third-party release tracker: Antigravity CLI 1.5.0 (November 4, 2026) added running "as an editor agent over ACP". [havoptic release tracker](https://www.havoptic.com/releases/antigravity-cli/2026/11)
- Search summary (not verified on a page fetch) says third-party (non-Gemini) models in Antigravity are offered only to certain clients (JetBrains, Android Studio, Zed, Xcode), while Gemini models work in every client. [Antigravity ACP docs](https://antigravity.google/docs/cli/acp/)
- Community ACP bridges for `agy` existed before the official server (agy-acp, agy-acp-bridge); one README carries a ToS risk warning. [jiridanek/agy-acp](https://github.com/jiridanek/agy-acp); [shindgew/agy-acp](https://github.com/shindgew/agy-acp)

### Inferences
- For FocusIDE's ACP mode, the Gemini-family agent to support in Nov 2026 is `agy acp` (consumer accounts) plus `gemini --acp` (API key and enterprise accounts). Both start the official binary, which keeps FocusIDE out of credential handling.
- Because `agy acp` reuses cached login from an interactive `agy` session, FocusIDE can tell users to sign in through the agent's own CLI in a terminal pane, which matches the "never touch credentials" rule.
- If non-Gemini models in Antigravity are limited to a client allowlist, FocusIDE as an unlisted ACP client would likely get Gemini models only.

### Gaps
- Exact date Gemini CLI moved from `--experimental-acp` to `--acp`, and whether the old flag is still accepted: not found.
- Zed's original August 2025 ACP launch blog post was not fetched in this session.
- Whether Antigravity CLI is open source: not stated in the Google blog (it links a GitHub repo described as a "community forum").

## Q3. Extension surfaces and turn start/end signals

### Takeaway
Gemini CLI has a hooks system configured in settings.json with events that map cleanly to turn boundaries: `BeforeAgent` (after prompt submit) and `AfterAgent` (agent loop ends), plus `SessionStart`/`SessionEnd`, tool events and `Notification`. Hooks speak JSON over stdin/stdout. Google says Antigravity CLI keeps Hooks, Skills, Subagents and Extensions (as "plugins").

### Cited Findings
- Hook events, "When It Fires": SessionStart "When a session begins (startup, resume, clear)"; SessionEnd "When a session ends (exit, clear)"; BeforeAgent "After user submits prompt, before planning"; AfterAgent "When agent loop ends"; BeforeModel "Before sending request to LLM"; AfterModel "After receiving LLM response"; BeforeToolSelection "Before LLM selects tools"; BeforeTool "Before a tool executes"; AfterTool "After a tool executes"; PreCompress "Before context compression"; Notification "When a system notification occurs". [Gemini CLI hooks](https://geminicli.com/docs/hooks/)
- Hooks are defined in `settings.json`, merged across project, user, system settings and installed extensions (project highest precedence). Input on stdin, output on stdout must be only a JSON object. Exit 0 success, 2 critical block, others warn. Changed project hooks are treated as untrusted and the user is warned before they run. [Gemini CLI hooks](https://geminicli.com/docs/hooks/)
- ACP `initialize` lets the client register its MCP server with the agent. [Gemini CLI ACP mode](https://geminicli.com/docs/cli/acp-mode/)
- Antigravity CLI keeps "Agent Skills, Hooks, Subagents, and Extensions (now as Antigravity plugins)", without "1:1 feature parity right out of the gate". [Google Developers Blog](https://developers.googleblog.com/en/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- Gemini extensions can be imported into Antigravity with `agy plugin import` (per a secondary guide). [Medium migration guide](https://medium.com/@hsinghungwang/migrating-from-gemini-cli-to-antigravity-cli-agy-af324c10c781)
- Usage statistics can be opted out via the configuration docs. [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)

### Inferences
- In TUI (terminal pane) mode, FocusIDE could get turn start/end from `BeforeAgent`/`AfterAgent` hooks writing to a local socket or file, the same pattern as Claude Code's hook events. This requires writing a hook into the user's or project's settings.json, which triggers Gemini CLI's untrusted-hook warning for project hooks.
- In ACP mode, turn boundaries come from the ACP `session/prompt` request and its response, so hooks are not needed.

### Gaps
- GEMINI.md context files, custom slash commands (TOML in `.gemini/commands`), MCP server config and OpenTelemetry telemetry exist in Gemini CLI per prior knowledge, but their doc pages were not fetched in this session. Verify before citing.
- Whether hooks are enabled by default: the hooks page does not say.
- Antigravity CLI hook event names and whether they match Gemini CLI's: not verified.

## Q4. Data use by auth mode

### Takeaway
Free (unpaid) Gemini API use lets Google use prompts and responses to improve products, with human review. Paid Gemini API, paid Code Assist plans and Code Assist Standard/Enterprise do not train on prompts. The free Code Assist for individuals tier (now discontinued for Gemini CLI) allowed opt-out.

### Cited Findings
- Gemini API Unpaid Services: "Google uses the content you submit to the Services and any generated responses to provide, improve, and develop" its products; "human reviewers may read, annotate, and process your API input and output."; "Do not submit sensitive, confidential, or personal information to the Unpaid Services." [Gemini API Additional Terms](https://aistudio.google.com/docs/terms)
- Gemini API Paid Services: "Google doesn't use your prompts ... or responses to improve our products"; "Google logs prompts and responses for a limited period of time". [Gemini API Additional Terms](https://aistudio.google.com/docs/terms)
- Gemini CLI FAQ: "Google does not use your data to improve Google's machine learning models if you purchase a paid plan." For free Code Assist for individuals, "you can also opt out of using your data to improve Google's machine learning models." [Gemini CLI FAQ](https://geminicli.com/docs/resources/faq/)
- Code Assist Standard/Enterprise: prompts and responses are not used to train Gemini Code Assist models; handled under Google Cloud terms and the Cloud Data Processing Addendum. Code customization stores private code. [Code Assist data governance](https://developers.google.com/gemini-code-assist/docs/data-governance); [Code Assist FAQ](https://docs.cloud.google.com/gemini/docs/codeassist/faqs)
- Vertex AI keys fall under Google Cloud Service Terms and the Cloud Privacy Notice. [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)
- Gemini CLI usage statistics go to Google under the Privacy Policy unless opted out. [tos-privacy.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)

### Inferences
- A useful one-line notice in FocusIDE: with a free Gemini API key, Google may use prompts and code for product improvement and humans may review it. Paid keys, Vertex AI and Code Assist Standard/Enterprise do not train on prompts.
- FocusIDE cannot see which auth mode the CLI uses (it does not read credentials), so any notice must be generic or based on what the agent reports over ACP `authenticate` / auth methods.

### Gaps
- Antigravity (consumer) data-use terms were not fetched; the old Code Assist individuals privacy notice now only shows the deprecation notice.
- Dates of the Code Assist data governance pages: not shown.
