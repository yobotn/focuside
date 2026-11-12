//! The agents FocusIDE starts over ACP: Claude Code through the bundled adapter, and other ACP
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

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// The adapter executable: FOCUSIDE_ACP_ADAPTER, then next to the app (installed sidecar),
/// then the dev copy in apps/desktop/src-tauri/binaries found from the app's folder upward.
pub fn locate_adapter() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FOCUSIDE_ACP_ADAPTER") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
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
        args: vec![],
        claude: Some(claude),
        env: vec![],
    })
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
}

pub const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        id: "claude",
        name: "Claude Code",
        program: "claude",
        args: &[],
        install: "npm install -g @anthropic-ai/claude-code",
    },
    AgentSpec {
        id: "codex",
        name: "Codex",
        program: "codex-acp",
        args: &[],
        install: "npm install -g @openai/codex @zed-industries/codex-acp",
    },
    AgentSpec {
        id: "gemini",
        name: "Gemini CLI",
        program: "gemini",
        args: &["--experimental-acp"],
        install: "npm install -g @google/gemini-cli",
    },
    AgentSpec {
        id: "opencode",
        name: "OpenCode",
        program: "opencode",
        args: &["acp"],
        install: "npm install -g opencode-ai",
    },
    AgentSpec {
        id: "qwen",
        name: "Qwen Code",
        program: "qwen",
        args: &["--experimental-acp"],
        install: "npm install -g @qwen-code/qwen-code",
    },
    AgentSpec {
        id: "goose",
        name: "Goose",
        program: "goose",
        args: &["acp"],
        install: "https://block.github.io/goose/docs/getting-started/installation",
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
        })
        .collect()
}

/// A program found on PATH as something `Command` can start: npm shims on Windows become
/// `node <script>`.
fn runnable(path: PathBuf, args: Vec<String>) -> (PathBuf, Vec<String>) {
    let real = which::resolve_shim(&path);
    let is_js = real
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("js") || e.eq_ignore_ascii_case("mjs"));
    if is_js {
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
            split_command("gemini --experimental-acp"),
            vec!["gemini", "--experimental-acp"]
        );
        assert_eq!(
            split_command(r#""C:\Program Files\x\agent.exe" acp  --flag "a b" ''"#),
            vec![r"C:\Program Files\x\agent.exe", "acp", "--flag", "a b", ""]
        );
        assert!(split_command("   ").is_empty());
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
