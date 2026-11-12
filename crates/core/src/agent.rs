//! Claude Code over ACP: finding the adapter and `claude`, and the session parameters FocusIDE
//! uses, including the plan format instruction.

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
