//! The agents FocusIDE starts over ACP: Claude Code through the claude-agent-acp adapter, and other ACP
//! agents the user installed. Also the session parameters, including the plan format instruction.

use crate::acp::AgentCommand;
use crate::which;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Appended to Claude Code's system prompt for every session. It only changes how plans end,
/// so the plan card can show them without a second model call. Nothing is written into the
/// user's repo.
pub const PLAN_INSTRUCTION: &str = "\
When you present a plan for approval (ExitPlanMode), keep your normal plan text, then end it \
with these three parts in exactly this format, in plain words:

Steps:
1. <first step>
2. <next step>
(6 steps or fewer)

Files:
create <path> - <short reason>
edit <path> - <short reason>
delete <path> - <short reason>
(one line for every file you will create, edit or delete; paths relative to the project root)

Risk: <one short line, or none>";

const ADAPTER: &str = "claude-agent-acp";

/// Adapter flag: it hides the claude.ai login method from `initialize` and refuses with
/// `authRequired` every session and turn that a Claude plan would pay for. Chat mode is built on
/// the Claude Agent SDK, so it takes API keys and cloud providers only.
pub const HIDE_CLAUDE_AUTH: &str = "--hide-claude-auth";

/// `data.reason` of the adapter's `authRequired` error when it refused a Claude plan login.
pub const SUBSCRIPTION_NOT_SUPPORTED: &str = "claude_subscription_not_supported";

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// The adapter executable: FOCUSIDE_ACP_ADAPTER, then the copy downloaded to app data
/// (`adapter_download`), then next to the app (older installs that bundled it), then the dev
/// copy in apps/desktop/src-tauri/binaries found from the app's folder upward.
pub fn locate_adapter() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FOCUSIDE_ACP_ADAPTER") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = crate::adapter_download::installed() {
        return Some(p);
    }
    let sidecar = exe(&format!("{ADAPTER}-{}", env!("FOCUSIDE_TARGET")));
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for name in [exe(ADAPTER), sidecar.clone()] {
        let p = exe_dir.join(&name);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut dir: Option<&Path> = Some(&exe_dir);
    while let Some(d) = dir {
        let p = d.join("apps/desktop/src-tauri/binaries").join(&sidecar);
        if p.is_file() {
            return Some(p);
        }
        dir = d.parent();
    }
    None
}

/// The user's own `claude`, following npm shims on Windows.
pub fn locate_claude() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FOCUSIDE_CLAUDE") {
        return Some(PathBuf::from(p));
    }
    which::find("claude").map(|p| which::resolve_shim(&p))
}

/// The command that starts Claude Code over ACP, or which part is missing.
pub fn claude_command() -> Result<AgentCommand, Missing> {
    let adapter = locate_adapter().ok_or(Missing::Adapter)?;
    let claude = locate_claude().ok_or(Missing::Claude)?;
    Ok(AgentCommand {
        adapter,
        args: vec![HIDE_CLAUDE_AUTH.to_string()],
        claude: Some(claude),
        env: vec![],
    })
}

/// How the user's Claude Code is signed in, from its own `claude auth status --json`. Only the
/// method and provider are read; no credential or account detail leaves Claude Code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAuth {
    pub logged_in: bool,
    /// For example "api_key", "third_party" or "claude.ai".
    pub method: String,
    /// "firstParty", "bedrock", "vertex", "foundry".
    pub provider: String,
}

/// Which kind of login Claude Code uses. Chat mode runs through the Claude Agent SDK (the
/// bundled adapter), and Anthropic asks SDK apps to use API key authentication. So chat mode
/// takes an API key (ANTHROPIC_API_KEY or apiKeyHelper) or a cloud provider. A Claude plan login
/// is not refused: it is routed to a Claude pane, the unmodified TUI (see `ClaudeRoute`). FocusIDE never
/// changes or hides a login method of the `claude` binary. See docs/agent-terms.md and
/// https://code.claude.com/docs/en/legal-and-compliance (Authentication and credential use).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeAccess {
    Allowed,
    /// Signed in with a Claude.ai account (Free, Pro, Max, Team, Enterprise) or its OAuth token.
    Subscription,
    SignedOut,
}

/// Where FocusIDE runs Claude for a given login. Chat mode is FocusIDE's own UI over ACP.
/// Pane means the unmodified `claude` TUI in a PTY in the center view (pane mode), with its
/// own login. The user types every prompt there; FocusIDE never types into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeRoute {
    Chat,
    Pane,
}

impl ClaudeAccess {
    pub fn route(self) -> ClaudeRoute {
        match self {
            ClaudeAccess::Allowed => ClaudeRoute::Chat,
            ClaudeAccess::Subscription | ClaudeAccess::SignedOut => ClaudeRoute::Pane,
        }
    }
}

impl ClaudeAuth {
    pub fn parse(json: &Value) -> Self {
        let s = |k: &str| json[k].as_str().unwrap_or_default().to_string();
        ClaudeAuth {
            logged_in: json["loggedIn"].as_bool().unwrap_or(false),
            method: s("authMethod"),
            provider: s("apiProvider"),
        }
    }

    pub fn access(&self) -> ClaudeAccess {
        if !self.logged_in {
            return ClaudeAccess::SignedOut;
        }
        let m = self.method.to_ascii_lowercase();
        let first_party = self.provider.is_empty() || self.provider == "firstParty";
        if first_party && (m.contains("claude.ai") || m.contains("oauth") || m.is_empty()) {
            return ClaudeAccess::Subscription;
        }
        ClaudeAccess::Allowed
    }
}

/// Runs `claude auth status --json` with the agent's environment.
pub async fn claude_auth(claude: &Path, env: &[(String, String)]) -> anyhow::Result<ClaudeAuth> {
    let mut cmd = tokio::process::Command::new(claude);
    cmd.args(["auth", "status", "--json"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    crate::proc::prepare(&mut cmd);
    let out = tokio::time::timeout(std::time::Duration::from_secs(20), cmd.output()).await??;
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text.find('{').unwrap_or(0);
    let json: Value = serde_json::from_str(&text[start..])?;
    Ok(ClaudeAuth::parse(&json))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Missing {
    Adapter,
    Claude,
    /// A non-Claude agent that is not installed (or an empty custom command).
    Agent,
}

/// Parameters for `session/new`. The plan instruction goes in as an appended system prompt
/// when the agent supports it.
pub fn new_session_params(cwd: &Path, system_prompt: bool) -> Value {
    let mut p = json!({ "cwd": cwd, "mcpServers": [] });
    if system_prompt {
        p["systemPrompt"] = json!({
            "mode": "append",
            "content": [{ "type": "text", "text": PLAN_INSTRUCTION }]
        });
    } else {
        // Older adapters read the same option from _meta.
        p["_meta"] = json!({ "systemPrompt": { "append": PLAN_INSTRUCTION } });
    }
    p
}

/// Parameters for `session/load` and `session/resume`.
pub fn load_session_params(cwd: &Path, session_id: &str, system_prompt: bool) -> Value {
    let mut p = new_session_params(cwd, system_prompt);
    p["sessionId"] = json!(session_id);
    p
}

/// An ACP agent FocusIDE can start. Claude Code goes through the bundled adapter; the others
/// speak ACP themselves (or through their own adapter) and must be installed by the user.
pub struct AgentSpec {
    pub id: &'static str,
    pub name: &'static str,
    /// Program to find on PATH.
    pub program: &'static str,
    pub args: &'static [&'static str],
    /// Shown when the program is missing.
    pub install: &'static str,
    /// The agent's own interactive TUI, for pane mode. None when the agent has none.
    pub tui_program: Option<&'static str>,
    /// Arguments for the TUI. Never a flag that picks a login method, never `-p`, `--bare` or
    /// `--append-system-prompt`: pane mode runs the agent as the user would in a terminal.
    pub tui_args: &'static [&'static str],
}

pub const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        id: "claude",
        name: "Claude",
        program: "claude",
        args: &[],
        install: "npm install -g @anthropic-ai/claude-code",
        // Verified: `claude` starts the interactive TUI (code.claude.com/docs/en/cli-reference).
        tui_program: Some("claude"),
        tui_args: &[],
    },
    AgentSpec {
        id: "codex",
        name: "Codex",
        program: "codex-acp",
        args: &[],
        install: "npm install -g @agentclientprotocol/codex-acp",
        // `codex` with no subcommand starts the TUI (Codex CLI docs). Not run on this machine.
        tui_program: Some("codex"),
        tui_args: &[],
    },
    AgentSpec {
        id: "gemini",
        name: "Gemini CLI",
        program: "gemini",
        args: &["--acp"],
        install: "npm install -g @google/gemini-cli",
        // `gemini` with no prompt starts the interactive CLI (Gemini CLI docs). Not run here.
        tui_program: Some("gemini"),
        tui_args: &[],
    },
    AgentSpec {
        id: "opencode",
        name: "OpenCode",
        program: "opencode",
        args: &["acp"],
        install: "npm install -g opencode-ai",
        // `opencode` with no subcommand starts the TUI (OpenCode docs). Not run here.
        tui_program: Some("opencode"),
        tui_args: &[],
    },
    AgentSpec {
        id: "qwen",
        name: "Qwen Code",
        program: "qwen",
        args: &["--acp", "--experimental-skills"],
        install: "npm install -g @qwen-code/qwen-code",
        // Unverified: `qwen` with no prompt starts the interactive CLI (a Gemini CLI fork).
        tui_program: Some("qwen"),
        tui_args: &[],
    },
    AgentSpec {
        id: "goose",
        name: "Goose",
        program: "goose",
        args: &["acp"],
        install: "https://block.github.io/goose/docs/getting-started/installation",
        // Verified in the docs: "goose session" starts an interactive session
        // (documentation/docs/guides/goose-cli-commands.md). Not run here.
        tui_program: Some("goose"),
        tui_args: &["session"],
    },
    // Phase 4 agents (2026-11-13). Each launch command was checked against the ACP registry
    // (cdn.agentclientprotocol.com/registry/v1/latest/registry.json) and the vendor docs on that
    // date; `node scripts/check-agents.mjs` repeats the registry part. None of them was run on
    // this machine. Install hints are commands or doc pages; FocusIDE never installs anything.
    AgentSpec {
        id: "antigravity",
        name: "Antigravity CLI",
        program: "agy",
        // Verified: antigravity.google/docs/cli/acp ("agy acp [flags]"). The registry entry
        // `antigravity-acp` is a different program, the `agy_acp_server` from the Antigravity
        // IDE extensions, so check-agents reports a mismatch on purpose.
        args: &["acp"],
        // The vendor installer is a script; the page shows it per OS.
        install: "https://antigravity.google/docs/install",
        // Verified: "Sign in once with an interactive `agy` session" (same page). Never `agy -p`.
        tui_program: Some("agy"),
        tui_args: &[],
    },
    AgentSpec {
        id: "copilot",
        name: "Copilot CLI",
        program: "copilot",
        // Verified: registry `github-copilot-cli` (npx @github/copilot --acp) and the GitHub
        // ACP server reference (public preview). stdio is the default transport.
        args: &["--acp"],
        // Verified: GitHub install page and the npm package's bin name `copilot`.
        install: "npm install -g @github/copilot",
        // Sign-in happens in the TUI (`/login`), which ACP does not support.
        tui_program: Some("copilot"),
        tui_args: &[],
    },
    AgentSpec {
        id: "cursor",
        name: "Cursor CLI",
        // The docs call the binary `agent`; the installer also creates `cursor-agent` (legacy
        // name, checked in cursor.com/install for both shells) and the registry starts
        // `cursor-agent acp`. `cursor-agent` is used because `agent` is too generic a name.
        // Unverified: starting the Windows `cursor-agent.cmd` (it runs a PowerShell script).
        program: "cursor-agent",
        // Verified: cursor.com/docs/cli/acp (`agent acp`) and registry `cursor`.
        args: &["acp"],
        // The vendor installer is a script (PowerShell on Windows, bash elsewhere).
        install: "https://cursor.com/docs/cli/installation",
        // Verified: running `agent` with no arguments starts interactive mode (install page).
        tui_program: Some("cursor-agent"),
        tui_args: &[],
    },
    AgentSpec {
        id: "kiro",
        name: "Kiro CLI",
        program: "kiro-cli",
        // Verified: kiro.dev/docs/cli/acp (`kiro-cli acp`). The registry adds
        // `--agent-engine=v3`, which overrides the user's saved engine setting, so it is left
        // out. The registry has no Windows build; the Kiro docs list Windows 11 as supported.
        args: &["acp"],
        install: "https://kiro.dev/docs/getting-started/installation/",
        // Verified: "run `kiro-cli`. The terminal UI opens" (kiro.dev/docs/cli/setup).
        tui_program: Some("kiro-cli"),
        tui_args: &[],
    },
    AgentSpec {
        id: "cline",
        name: "Cline",
        program: "cline",
        // Verified: registry `cline` (npx cline --acp) and docs.cline.bot ACP page.
        args: &["--acp"],
        install: "npm install -g cline",
        // Verified: `cline` with no arguments opens an interactive session (Cline CLI docs).
        tui_program: Some("cline"),
        tui_args: &[],
    },
    AgentSpec {
        id: "kimi",
        name: "Kimi Code",
        // The npm package's bin is `kimi`. The archived Kimi CLI (legacy) used the same name.
        program: "kimi",
        // Verified in the registry only (`kimi-code`: npx @moonshot-ai/kimi-code acp). The
        // Kimi Code site does not document ACP mode.
        args: &["acp"],
        install: "npm install -g @moonshot-ai/kimi-code",
        // Verified: "run kimi inside any project to start a conversation" (Kimi Code site).
        tui_program: Some("kimi"),
        tui_args: &[],
    },
    AgentSpec {
        id: "vibe",
        name: "Mistral Vibe",
        // Verified: registry `mistral-vibe` (`vibe-acp`, no arguments) and the `vibe-acp`
        // script in the package's pyproject.toml.
        program: "vibe-acp",
        args: &[],
        // Verified: README ("uv tool install mistral-vibe"); installs `vibe` and `vibe-acp`.
        install: "uv tool install mistral-vibe",
        // Verified: "run `vibe` in your project" (README).
        tui_program: Some("vibe"),
        tui_args: &[],
    },
];

/// What the agent picker shows: each known agent and whether it is installed.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub install: String,
    /// The agent's TUI is installed, so it can open in a pane.
    pub tui: bool,
}

pub fn agents() -> Vec<AgentInfo> {
    AGENTS
        .iter()
        .map(|a| AgentInfo {
            id: a.id.into(),
            name: a.name.into(),
            path: if a.id == "claude" {
                locate_claude()
            } else {
                which::find(a.program)
            },
            install: a.install.into(),
            tui: tui_command(a.id).is_ok(),
        })
        .collect()
}

/// An agent's own TUI, resolved for a PTY: the program, its arguments, and nothing else. The
/// environment is the app's own (no variable added or removed, so the agent's login stays the
/// one the user set up).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// Flags pane mode never passes: they force a login method, skip the user's setup, run without
/// a human, or put FocusIDE text into the model context.
pub const FORBIDDEN_TUI_FLAGS: &[&str] = &[
    "-p",
    "--print",
    "--bare",
    "--append-system-prompt",
    "--system-prompt",
    "--settings",
];

/// The TUI command for an agent id, found with the same lookup as chat mode (PATH through
/// `which`, npm shims followed, `node <script>` when the shim points at a script).
pub fn tui_command(id: &str) -> Result<TuiCommand, Missing> {
    let spec = AGENTS
        .iter()
        .find(|a| a.id == id || (id.is_empty() && a.id == "claude"))
        .ok_or(Missing::Agent)?;
    let program = spec.tui_program.ok_or(Missing::Agent)?;
    let path = if spec.id == "claude" {
        locate_claude().ok_or(Missing::Claude)?
    } else {
        which::find(program).ok_or(Missing::Agent)?
    };
    let args = spec.tui_args.iter().map(|s| s.to_string()).collect();
    let (program, args) = runnable(path, args);
    Ok(TuiCommand { program, args })
}

/// A program found on PATH as something `Command` can start: npm shims on Windows become
/// `node <script>`.
fn runnable(path: PathBuf, args: Vec<String>) -> (PathBuf, Vec<String>) {
    let real = which::resolve_shim(&path);
    if which::is_node_script(&real) {
        if let Some(node) = which::find("node") {
            let mut a = vec![real.to_string_lossy().to_string()];
            a.extend(args);
            return (node, a);
        }
    }
    (real, args)
}

/// Splits a command line on spaces, keeping "quoted parts" together.
pub fn split_command(line: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut any = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                any = true;
            }
            (None, c) if c.is_whitespace() => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The command for an agent id. `custom` is the user's own command line for id "custom".
pub fn agent_command(id: &str, custom: Option<&str>) -> Result<AgentCommand, Missing> {
    if id == "claude" || id.is_empty() {
        return claude_command();
    }
    let mut env = vec![];
    let (program, args): (String, Vec<String>) = if id == "custom" {
        let mut parts = split_command(custom.unwrap_or_default());
        // Leading NAME=value parts set environment variables, as in a shell.
        while let Some((k, v)) = parts.first().and_then(|p| env_pair(p)) {
            env.push((k, v));
            parts.remove(0);
        }
        if parts.is_empty() {
            return Err(Missing::Agent);
        }
        let p = parts.remove(0);
        (p, parts)
    } else {
        let spec = AGENTS.iter().find(|a| a.id == id).ok_or(Missing::Agent)?;
        (
            spec.program.to_string(),
            spec.args.iter().map(|s| s.to_string()).collect(),
        )
    };
    let path = if Path::new(&program).is_absolute() {
        Some(PathBuf::from(&program)).filter(|p| p.is_file())
    } else {
        which::find(&program)
    };
    let path = path.ok_or(Missing::Agent)?;
    let (adapter, args) = runnable(path, args);
    Ok(AgentCommand {
        adapter,
        args,
        claude: None,
        env,
    })
}

/// "NAME=value" as an environment pair, when NAME is a valid variable name.
fn env_pair(part: &str) -> Option<(String, String)> {
    let (k, v) = part.split_once('=')?;
    let valid = !k.is_empty()
        && !k.starts_with(|c: char| c.is_ascii_digit())
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then(|| (k.to_string(), v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_command_lines() {
        assert_eq!(
            split_command("qwen --acp --experimental-skills"),
            vec!["qwen", "--acp", "--experimental-skills"]
        );
        assert_eq!(
            split_command(r#""C:\Program Files\x\agent.exe" acp  --flag "a b" ''"#),
            vec![r"C:\Program Files\x\agent.exe", "acp", "--flag", "a b", ""]
        );
        assert!(split_command("   ").is_empty());
    }

    #[test]
    fn claude_access_rules() {
        let a = |j: Value| ClaudeAuth::parse(&j).access();
        assert_eq!(
            a(json!({"loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty"})),
            ClaudeAccess::Subscription
        );
        assert_eq!(
            a(json!({"loggedIn": true, "authMethod": "oauth_token", "apiProvider": "firstParty"})),
            ClaudeAccess::Subscription
        );
        assert_eq!(
            a(json!({"loggedIn": true, "authMethod": "api_key", "apiProvider": "firstParty"})),
            ClaudeAccess::Allowed
        );
        assert_eq!(
            a(json!({"loggedIn": true, "authMethod": "third_party", "apiProvider": "bedrock"})),
            ClaudeAccess::Allowed
        );
        assert_eq!(a(json!({"loggedIn": false})), ClaudeAccess::SignedOut);
        assert_eq!(ClaudeAccess::Allowed.route(), ClaudeRoute::Chat);
        assert_eq!(ClaudeAccess::Subscription.route(), ClaudeRoute::Pane);
        assert_eq!(ClaudeAccess::SignedOut.route(), ClaudeRoute::Pane);
    }

    /// Runs the real `claude auth status` on this machine: `cargo test -- --ignored real_claude`.
    #[tokio::test]
    #[ignore]
    async fn real_claude_access() {
        let claude = locate_claude().expect("claude on PATH");
        let auth = claude_auth(&claude, &[]).await.unwrap();
        println!(
            "method={} provider={} access={:?}",
            auth.method,
            auth.provider,
            auth.access()
        );
        let with_key = claude_auth(
            &claude,
            &[("ANTHROPIC_API_KEY".into(), "sk-ant-test-not-real".into())],
        )
        .await
        .unwrap();
        assert_eq!(with_key.access(), ClaudeAccess::Allowed);
    }

    #[test]
    fn tui_commands_pass_no_forbidden_flags() {
        for a in AGENTS {
            assert!(a.tui_program.is_some(), "{} has a TUI", a.id);
            for arg in a.tui_args {
                assert!(
                    !FORBIDDEN_TUI_FLAGS.contains(arg),
                    "{} passes {arg} in pane mode",
                    a.id
                );
            }
        }
        let claude = AGENTS.iter().find(|a| a.id == "claude").unwrap();
        assert_eq!(claude.tui_program, Some("claude"));
        assert!(claude.tui_args.is_empty());
        assert_eq!(tui_command("custom"), Err(Missing::Agent));
        assert_eq!(tui_command("nope"), Err(Missing::Agent));
    }

    #[test]
    fn agent_ids_are_unique_and_never_print_mode() {
        let mut ids: Vec<_> = AGENTS.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), AGENTS.len());
        assert!(!ids.contains(&"custom"));
        for a in AGENTS {
            assert!(!a.install.is_empty(), "{} has an install hint", a.id);
            for arg in a.args {
                assert!(!["-p", "--print"].contains(arg), "{} runs headless", a.id);
            }
        }
    }

    #[test]
    fn env_prefixes() {
        assert_eq!(
            env_pair("API_KEY=a=b"),
            Some(("API_KEY".into(), "a=b".into()))
        );
        assert_eq!(env_pair("--flag=x"), None);
        assert_eq!(env_pair(r"C:\x=y"), None);
    }
}
