# Open source coding agents and the ACP ecosystem (for a third-party desktop client), as of 12 Nov 2026

## Agent Client Protocol: spec version, governance, registry, agents, clients

### Takeaway
ACP is jointly governed by Zed and JetBrains (JetBrains' Sergey Ignatov became Lead Maintainer in Feb 2026). The stable line is v1 plus incrementally stabilized RFD features; "ACP v2" has been in draft since 20 Jul 2026 and the Rust and TypeScript SDKs reached 1.0 on 25 Jun 2026. A shared public agent registry (launched 28 Jan 2026, stabilized 9 Mar 2026) lists 45 agents as of Nov 2026, each with license and distribution type (npx, binary, uvx).

### Cited Findings
- Governance: ACP is jointly governed by Zed and JetBrains under an "interim governance model"; Sergey Ignatov (JetBrains) joined as Lead Maintainer, announced Feb 2026 - [ACP governance](https://agentclientprotocol.com/community/governance.md); [ACP announcement](https://agentclientprotocol.com/announcements/sergey-ignatov-lead-maintainer)
- JetBrains joined ACP in Oct 2025 - [Tessl, 23 Oct 2025](https://tessl.io/blog/jetbrains-joins-agent-client-protocol-to-open-ai-coding-agents-to-any-ide/); JetBrains says it is "not playing favorites" among agents - [JetBrains AI blog, Dec 2025](https://blog.jetbrains.com/ai/2025/12/agents-protocols-and-why-we-re-not-playing-favorites/)
- ACP dated updates (newest first): 6 Nov 2026 Client-Provided System Prompts stabilized; 17 Sep 2026 Tool Call Names stabilized; 22 Jul 2026 Elicitation stabilized; 20 Jul 2026 "ACP v2 is Available in Draft"; 6 Jul 2026 Boolean Config Options; 29 Jun 2026 Request Cancellation; 25 Jun 2026 "Rust and TypeScript SDKs Reach 1.0"; 24 Jun 2026 Model Config Category; 5 Jun 2026 Message IDs and Session Usage Updates; 1 Jun 2026 Session Delete and Additional Workspace Roots; 21 May 2026 Logout Method; 23 Apr 2026 Session Close; 22 Apr 2026 Session Resume and a Transports Working Group formed; 9 Mar 2026 ACP Registry released and Session Info Update; Feb 2026 Session List; 24 Oct 2025 Session Config Options - [ACP updates page](https://agentclientprotocol.com/updates)
- No single named "2026 spec version number" was found beyond v1 (stable) and v2 (draft) - [ACP updates page](https://agentclientprotocol.com/updates)
- Registry: launched by Zed and JetBrains on 28 Jan 2026 (beta); RFD completed and registry stabilized 9 Mar 2026 - [Zed blog: The ACP Registry is Live](https://zed.dev/blog/acp-registry); [ACP announcement, 9 Mar 2026](https://agentclientprotocol.com/announcements/acp-agent-registry-stabilized)
- Registry only lists agents that support user authentication; CI verifies each agent returns valid `authMethods` in the ACP handshake (per search summary of registry/Zed sources) - [Zed blog](https://zed.dev/blog/acp-registry); registry code is Apache-2.0, agents keep their own licenses - [awesome.ecosyste.ms entry for agentclientprotocol/registry](https://awesome.ecosyste.ms/projects/github.com%2Fagentclientprotocol%2Fregistry)
- Registry index URL: `https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json` (format version 1.0.0). Fetched 12 Nov 2026, 45 entries with license and distribution, including: claude-acp "Claude Agent" 0.106.0 (binary, npx, proprietary); codex-acp 2.6.0 (npx, Apache-2.0); gemini 0.68.0 (npx, Apache-2.0); github-copilot-cli 1.0.109 (npx, proprietary); goose 1.59.0 (binary, Apache-2.0); cline 3.0.74 (npx, Apache-2.0); opencode and opencode-v2 2.2.7 (binary, MIT); qwen-code 0.27.2 (npx, Apache-2.0); kimi-code 2.6.0 (npx, MIT) with "Kimi CLI (Legacy)" 1.52.0 marked archived; mistral-vibe 3.1.1 (binary, Apache-2.0); kilo 7.8.27 (MIT); pi-acp 0.0.34 (MIT); auggie 0.37.0 (npx, proprietary); cursor, devin, junie, kiro, factory-droid, antigravity-acp, grok-build, poolside, qoder (proprietary); amp-acp (Apache-2.0); corust-agent (GPL-3.0-or-later); minion-code (AGPL-3.0); fast-agent (uvx, Apache 2.0) - [registry.json latest](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- Agents page: adapter-based (labelled "via"): Claude Agent (via Zed's SDK adapter), Codex CLI (via adapter), Pi (via pi-acp), Bub, acp-llm-adapter. Others without adapter label (likely native): Augment Code, Cline, Cursor, Docker cagent, Factory Droid, Gemini CLI, GitHub Copilot (public preview), Goose, Junie, Kimi CLI, Kiro CLI, Mistral Vibe, OpenClaw, OpenCode, OpenHands, Poolside, Qoder CLI, Qwen Code, Stakpak, VT Code, and more - [ACP agents page](https://agentclientprotocol.com/overview/agents)
- Clients (editors/IDEs section): Zed, JetBrains, Neovim (CodeCompanion, agentic.nvim, avante.nvim), Emacs agent-shell.el, VS Code (several extensions), Visual Studio (Poolside), Qt Creator, Sublime Text, Pulsar, Obsidian plugins, Unity plugins, Athas, Anycode, Volt, Chrome extensions, and others; the page also has separate CLI/TUI, desktop/web and mobile client sections (not enumerated here) - [ACP clients page](https://agentclientprotocol.com/overview/clients)
- JetBrains Central / IntelliJ IDEA 2026.1 lets users run Cursor, Codex, Claude Agent and Copilot via ACP - [Machine Herald, Mar 2026 (secondary)](https://machineherald-io.pages.dev/article/2026-03/25-jetbrains-unveils-central-a-control-plane-for-agentic-development-as-intellij-20261-opens-the-ide-to-cursor-codex-and-any-acp-compatible-agent); WebStorm ACP post - [JetBrains WebStorm blog, Aug 2026](https://blog.jetbrains.com/webstorm/2026/08/the-lsp-moment-for-ai-agents-webstorm-acp/)
- AWS Kiro rebuilt its harnesses around ACP (Aug 2026) - [The New Stack](https://thenewstack.io/kiro-agent-client-protocol/)

### Inferences
- FocusIDE can consume the registry JSON to offer agent install/launch, and should handle `authMethods` from the handshake (the registry requires them), which fits the "never touch credentials" rule because the agent itself runs its login.
- Recent stabilized features (session resume/close/delete, logout, elicitation, usage updates, client-provided system prompts) are worth tracking; v2 draft may break wire compat later.

### Gaps
- Exact current stable protocol version integer and the v2 draft contents were not fetched.
- Registry submission/approval rules (who approves) not documented on the pages read.

## Open source agents: license, ACP, auth model, hooks/plugins

### Takeaway
Most open agents are MIT or Apache-2.0 and speak ACP natively (OpenCode, Goose, Cline, Qwen Code, Kimi, Mistral Vibe, OpenHands, Gemini CLI). Aider has no shipped ACP (third-party AiderDesk does). Crush is FSL-1.1-MIT (source-available, not OSI). Auggie is proprietary. Auth is mostly bring-your-own-key, plus some provider logins: GitHub Copilot officially supports OpenCode (Jan 2026); ChatGPT/Codex login is offered by Cline and OpenCode plugins; Claude Pro/Max OAuth was removed from OpenCode (Feb 2026); Qwen OAuth free tier ended 15 Apr 2026.

### Cited Findings
- OpenCode: MIT, ACP agent in registry (binary, versions opencode and opencode-v2 at 2.2.7) - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json). Plugin system: plugins in `opencode.json` `plugins` array (npm, git, local dirs), auto-loaded from `.opencode/plugins/` and `~/.config/opencode/plugins/`, can add "tools, hooks, integrations, commands, agents"; `opencode plugin add/list/check/update/remove`; desktop GUI plugins via `./gui-experimental` - [OpenCode plugins docs](https://opencode.ai/docs/plugins/)
- OpenCode auth: GitHub officially supports using Copilot Pro, Pro+, Business, Enterprise subscriptions with OpenCode (formal partnership, Jan 2026) - [GitHub changelog 2026-01-16](https://github.blog/changelog/2026-01-16-github-copilot-now-supports-opencode/); ChatGPT Plus/Pro login via community plugin - [numman-ali/opencode-openai-codex-auth](https://github.com/numman-ali/opencode-openai-codex-auth); Claude Pro/Max removed in a commit citing "anthropic legal requests" (~19 Feb 2026) - [The Register, 20 Feb 2026](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access)
- Goose (Block): Apache-2.0, ACP native, registry binary 1.59.0 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json); listed as ACP agent - [ACP agents page](https://agentclientprotocol.com/overview/agents)
- Cline: Apache-2.0, ACP via npx 3.0.74 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json); offers an "OpenAI Codex" provider that signs in via OAuth and uses the user's ChatGPT subscription - [Cline docs: OpenAI Codex](https://docs.cline.bot/provider-config/openai-codex)
- Aider: no ACP release found; Zed said an Aider ACP implementation was "underway" (Oct 2025) - [Zed ACP progress report](https://zed.dev/blog/acp-progress-report); AiderDesk (separate project) ships `aiderdesk acp` - [AiderDesk ACP docs](https://aiderdesk.hotovo.com/docs/advanced/acp)
- Qwen Code: Apache-2.0, ACP via npx 0.27.2 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json). Qwen OAuth free tier discontinued 15 Apr 2026 (after cut from 1,000 to 100 req/day); now API key or Alibaba Cloud Coding Plan - [Decrypt](https://decrypt.co/364501/alibaba-shuts-down-free-tier-qwen-code); [Qwen Code auth docs](https://cdn.jsdelivr.net/npm/@qwen-code/qwen-code@0.17.0/bundled/qc-helper/docs/configuration/auth.md)
- Kimi (Moonshot): "Kimi CLI (Legacy)" 1.52.0 archived, replaced by "Kimi Code" 2.6.0 (npx), both MIT - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- Mistral Vibe: Apache-2.0, ACP binary 3.1.1 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- OpenHands: listed as ACP agent - [ACP agents page](https://agentclientprotocol.com/overview/agents); not present in the registry JSON fetched 12 Nov 2026 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- Crush (Charm): FSL-1.1-MIT, source-available, each release converts to MIT after two years; restricts competing commercial use until then; not OSI-approved for current releases - [Charm docs](https://next-docs.charm.land/crush/); [license commit mirror](https://git.secluded.site/crush/commit/2562b0dc79221a85c5e637d0a003a2b3ba656644)
- Augment Auggie CLI: proprietary license, ACP via npx 0.37.0 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- Gemini CLI: Apache-2.0, ACP via npx 0.68.0 - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- Codex: codex-acp adapter is Apache-2.0 (npx 2.6.0) - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json); Claude Agent (claude-acp) registry entry is marked proprietary - same source

### Inferences
- For FocusIDE, the open agents with native ACP and permissive licenses (OpenCode, Goose, Cline, Qwen Code, Kimi Code, Mistral Vibe, Gemini CLI) are the low-friction set. Crush should not be described as "open source" in FocusIDE copy.

### Gaps
- Per-agent hook systems for Goose (extensions/MCP), Cline (hooks), Qwen Code, Kimi, Mistral Vibe and OpenHands were not verified from primary docs this round.
- OpenHands license (core MIT, with a separately licensed enterprise directory per earlier knowledge) not verified this round.
- Licenses of Aider (Apache-2.0 per prior knowledge) and OpenHands not confirmed with a fetched source.
- Whether Goose or Kimi offer any provider subscription login was not checked.

## Licensing implications for a GPL-3.0 app

### Takeaway
Spawning a separate agent binary and talking to it over stdio JSON-RPC (ACP) is, per the FSF's own FAQ, communication between separate programs, so the agent's license does not need to be GPL-compatible. Bundling (shipping the binary in the installer) is "mere aggregation" if they stay separate programs, but you must still honor each bundled component's license and redistribution terms; a proprietary binary (e.g. the Claude Agent adapter if marked proprietary, Auggie, Cursor) needs explicit redistribution permission. MIT and Apache-2.0 code may also be linked into GPLv3 code.

### Cited Findings
- FSF: "An 'aggregate' consists of a number of separate programs, distributed together on the same CD-ROM or other media"; pipes, sockets and command-line arguments are "communication mechanisms normally used between two separate programs"; fork/exec "without intimate communication" means a separate program; exchanging complex internal data structures could make it one combined program - [GNU GPL FAQ #MereAggregation, #GPLPlugins](https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation)
- Apache-2.0 is listed by the FSF as compatible with GPLv3 (not GPLv2) - [FSF license list, Apache 2.0](https://www.gnu.org/licenses/license-list.html#apache2)
- ACP registry entries carry a license field per agent; claude-acp is labelled "proprietary" in the registry - [registry.json](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)

### Inferences
- Spawning a user-installed binary: no redistribution, so the agent's license terms on distribution do not bind FocusIDE; the user's own agreement with the agent vendor and model provider applies.
- Bundling an MIT/Apache binary: allowed; include its LICENSE/NOTICE files (Apache-2.0 requires NOTICE retention). FocusIDE's GPL does not extend to it.
- Bundling a "proprietary" entry (check the actual upstream LICENSE of `@agentclientprotocol/claude-agent-acp` / Zed adapter, since the registry label may refer to the Claude Code dependency rather than the adapter source) needs permission; FocusIDE currently bundles `claude-agent-acp`, so verify its LICENSE file and the Claude Code / Agent SDK commercial terms.
- FSL-1.1 (Crush) bundling inside a product that "competes" could breach FSL; spawning a user-installed copy is safer.

### Gaps
- Not legal advice; did not verify the upstream adapter repo's LICENSE file or Anthropic's Agent SDK redistribution terms in this round.

## Commercial subscription logins in open source agents: whose rules apply, and blocks

### Takeaway
The provider's terms govern use of its subscription regardless of the agent's open source license. Anthropic forbids Claude Free/Pro/Max OAuth tokens in any third-party tool (blocked technically 9 Jan 2026, clarified in docs ~19 to 20 Feb 2026, subscription limits removed for third-party harnesses from 4 Apr 2026, programmatic credits regime from 15 Jun 2026). OpenAI publicly endorses ChatGPT/Codex sign-in in third-party harnesses. GitHub officially partners with OpenCode for Copilot login. Google has suspended Gemini CLI / Antigravity accounts used via third-party tools like OpenClaw and OpenCode.

### Cited Findings
- 9 Jan 2026: Anthropic deployed server-side checks blocking third-party tools from authenticating with Pro/Max OAuth; Anthropic's Thariq Shihipar said they stopped tools "spoofing the Claude Code harness" - [VentureBeat](https://venturebeat.com/technology/anthropic-cracks-down-on-unauthorized-claude-usage-by-third-party-harnesses); [The Register](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access)
- ~20 Feb 2026: Anthropic compliance page: "Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service" (including the Agent SDK) is not permitted; Consumer Terms section 3.7 (since 2024) bars unendorsed automated access "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it"; OAuth is for Claude Code and Claude.ai only; reasons: "token arbitrage" and unusual traffic without telemetry - [The Register, 20 Feb 2026](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access)
- OpenCode removed Claude Pro/Max support citing "anthropic legal requests" (~19 Feb 2026) - [The Register](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access)
- 4 Apr 2026 (12pm PT): subscribers can "no longer ... use your Claude subscription limits for third-party harnesses including OpenClaw"; must use pay-as-you-go extra usage; applies "to all third-party harnesses and will be rolled out to more shortly"; one-time credit equal to monthly plan, redeem by 17 Apr - [TechCrunch, 4 Apr 2026](https://techcrunch.com/2026/04/04/anthropic-says-claude-code-subscribers-will-need-to-pay-extra-for-openclaw-support/); [InfoWorld](https://www.infoworld.com/article/4154435/anthropic-cuts-openclaw-access-from-claude-subscriptions-offers-credits-to-ease-transition.html)
- ~13 May 2026 announced, live 15 Jun 2026: paid plans get monthly credits for "programmatic" use billed at API rates; `claude -p` and Agent SDK treated as programmatic; interactive use of Anthropic front-ends stays in subscription (opinion piece, secondary) - [Kilo blog](https://blog.kilo.ai/p/the-subsidy-was-never-about-you)
- Account penalties: conflicting reports; some say accounts disabled, others say bans tied to multiple Max accounts rather than tool use - [falcao.org (secondary)](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/)
- OpenAI: Thibault Sottiaux publicly endorsed using Codex subscriptions in third-party harnesses (Feb 2026) - [The Register](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access); "Sign in with ChatGPT" for third-party tools (Amp, Devin, OpenClaw) with per-app weekly caps, Sep 2026 - [The New Stack](https://thenewstack.io/sign-in-with-chatgpt/) (page returned 403 to fetch; summary from search); OpenAI auth docs - [developers.openai.com/docs/auth](https://developers.openai.com/de-DE/docs/auth)
- GitHub: official Copilot subscription support in OpenCode, Jan 2026 - [GitHub changelog](https://github.blog/changelog/2026-01-16-github-copilot-now-supports-opencode/); org admins may need to approve the OAuth app - [AI EngineerGuide](https://aiengineerguide.com/til/github-copilot-with-opencode/)
- Google: mass 403 "ToS violation" suspensions of Antigravity / Gemini CLI accounts used through OpenClaw (Feb 2026), Google staff later mentioned a "system-wide automated unban"; repeat offenses warned to be permanent - [Google AI dev forum thread](https://discuss.ai.google.dev/t/urgent-mass-403-tos-bans-on-gemini-api-antigravity-for-open-source-cli-users-paid-tier/124508); [The Register, 23 Feb 2026](https://www.theregister.com/2026/02/23/google_antigravity_compute_burden/); further reports of suspensions for third-party tools incl. OpenCode (Sep 2026) - [Enterprise DNA](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-09-03-google-suspends-paying-antigravity-subscribers-for-using-thi)
- Alibaba: Qwen OAuth free tier ended 15 Apr 2026 - [Decrypt](https://decrypt.co/364501/alibaba-shuts-down-free-tier-qwen-code)

### Inferences
- For FocusIDE: driving Claude Code itself (via the ACP adapter running the user's own Claude Code login) is the sanctioned path, but under the Feb 2026 wording "including the Agent SDK", an SDK-based adapter using subscription OAuth is a grey area; the 15 Jun 2026 regime suggests SDK use draws on programmatic credits. Worth a primary-source check of Anthropic's current compliance page.
- FocusIDE never touching credentials is good, but the user's account risk still depends on what the spawned agent does; FocusIDE copy should state that the provider's terms apply and recommend API keys or provider-sanctioned logins (Copilot for OpenCode, ChatGPT sign-in).
- Embedding the vendor's own CLI TUI in a terminal pane is the least ambiguous way to use a subscription, since it is the vendor's own front-end.

### Gaps
- Anthropic's primary compliance page text and the 15 Jun 2026 credits announcement were not fetched directly; details rely on press.
- Whether a third-party GUI driving the official Claude Code binary (not SDK) counts as "third-party harness" under the 4 Apr policy is not explicitly addressed in sources found.
- OpenAI's official list of approved "Sign in with ChatGPT" partners and terms not fetched.
