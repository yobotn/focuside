//! FocusIDE hook helper. An agent (Claude Code through the `focuside` plugin, or Codex and
//! Gemini CLI through hooks the user adds) runs it on a hook event with the event JSON on
//! stdin. It writes a few fields of the event as one JSON file into the folder named by
//! `FOCUSIDE_EVENTS`, which FocusIDE sets for the agent panes it starts.
//!
//! Rules (docs/agent-terms.md, "Pane-mode risks"): it prints nothing to stdout or stderr,
//! always exits 0, reads no environment variable except `FOCUSIDE_EVENTS`, never reads
//! `transcript_path`, and writes only the fields FocusIDE uses. Hook stdout can reach the
//! model on some events, so silence is what keeps the hook observe-only.

use serde_json::{json, Map, Value};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

/// The event file format. FocusIDE ignores fields it does not know.
pub const SCHEMA: u64 = 1;

/// The largest stdin read. Longer input is cut, which makes it invalid JSON and ignored.
const MAX_INPUT: u64 = 16 * 1024 * 1024;
/// The longest plan and last message kept, in bytes.
const MAX_TEXT: usize = 64 * 1024;
const MAX_SHORT: usize = 4 * 1024;

/// Canonical event name for a hook event of `agent`. Gemini CLI names its events differently.
/// None for events FocusIDE does not use.
fn canonical(agent: &str, event: &str) -> Option<&'static str> {
    let e = match (agent, event) {
        ("gemini", "BeforeAgent") => "UserPromptSubmit",
        ("gemini", "AfterAgent") => "Stop",
        ("gemini", "BeforeTool") => "PreToolUse",
        ("gemini", "AfterTool") => "PostToolUse",
        (_, "UserPromptSubmit") => "UserPromptSubmit",
        (_, "Stop") => "Stop",
        (_, "StopFailure") => "StopFailure",
        (_, "PreToolUse") => "PreToolUse",
        (_, "PostToolUse") => "PostToolUse",
        _ => return None,
    };
    Some(e)
}

/// Cuts `s` to at most `max` bytes on a char boundary.
fn cut(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

fn text(v: &Value, max: usize) -> Option<Value> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .map(|s| Value::String(cut(s, max)))
}

/// The agent name from the first argument: lowercase letters only, else "claude".
pub fn agent_arg(arg: Option<&str>) -> String {
    match arg {
        Some(a) if !a.is_empty() && a.len() <= 20 && a.bytes().all(|b| b.is_ascii_lowercase()) => {
            a.to_string()
        }
        _ => "claude".to_string(),
    }
}

/// The event FocusIDE stores for one hook input, or None when the input is not JSON or the
/// event is not one FocusIDE uses.
pub fn event_from(input: &[u8], agent: &str) -> Option<Value> {
    let v: Value = serde_json::from_slice(input).ok()?;
    let name = canonical(agent, v.get("hook_event_name")?.as_str()?)?;
    let mut out = Map::new();
    out.insert("v".into(), json!(SCHEMA));
    out.insert("agent".into(), json!(agent));
    out.insert("event".into(), json!(name));
    if let Some(s) = text(&v["session_id"], 200) {
        out.insert("sessionId".into(), s);
    }
    match name {
        "Stop" => {
            // Gemini CLI calls the last answer `prompt_response`.
            let msg = if v["last_assistant_message"].is_string() {
                &v["last_assistant_message"]
            } else {
                &v["prompt_response"]
            };
            if let Some(s) = text(msg, MAX_TEXT) {
                out.insert("lastMessage".into(), s);
            }
        }
        "StopFailure" => {
            if let Some(s) = text(&v["error"], 200) {
                out.insert("error".into(), s);
            }
        }
        "PreToolUse" | "PostToolUse" => {
            let tool = v["tool_name"].as_str().unwrap_or("");
            if !tool.is_empty() {
                out.insert("tool".into(), json!(cut(tool, 200)));
            }
            let input = &v["tool_input"];
            if tool == "ExitPlanMode" {
                // Claude Code adds the plan to tool_input; PostToolUse also has the approved
                // plan in tool_response.
                let plan = if name == "PostToolUse" && v["tool_response"]["plan"].is_string() {
                    &v["tool_response"]["plan"]
                } else {
                    &input["plan"]
                };
                if let Some(s) = text(plan, MAX_TEXT) {
                    out.insert("plan".into(), s);
                }
            }
            for key in ["file_path", "notebook_path", "path"] {
                if let Some(s) = text(&input[key], MAX_SHORT) {
                    out.insert("filePath".into(), s);
                    break;
                }
            }
            let cmd = match &input["command"] {
                Value::Array(parts) => Value::String(
                    parts
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                other => other.clone(),
            };
            if let Some(s) = text(&cmd, MAX_SHORT) {
                out.insert("command".into(), s);
            }
        }
        _ => {}
    }
    Some(Value::Object(out))
}

/// Writes `event` as a new file in `dir`: first a temporary name, then a rename, so a reader
/// never sees half a file. Names sort by time.
pub fn write_event(dir: &Path, event: &Value) -> std::io::Result<()> {
    static N: AtomicU32 = AtomicU32::new(0);
    std::fs::create_dir_all(dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!(
        "{nanos:024}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    );
    let tmp = dir.join(format!("{name}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec(event).unwrap_or_default())?;
    std::fs::rename(&tmp, dir.join(format!("{name}.json"))).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// The whole helper: read the input, write the event. Every error is dropped.
pub fn run(events_dir: Option<&Path>, agent: &str, stdin: impl Read) {
    let Some(dir) = events_dir.filter(|d| !d.as_os_str().is_empty()) else {
        return;
    };
    let mut input = Vec::new();
    if stdin.take(MAX_INPUT).read_to_end(&mut input).is_err() {
        return;
    }
    if let Some(ev) = event_from(&input, agent) {
        let _ = write_event(dir, &ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(input: Value, agent: &str) -> Value {
        event_from(input.to_string().as_bytes(), agent).expect("event")
    }

    #[test]
    fn claude_events() {
        let e = ev(
            json!({"hook_event_name": "UserPromptSubmit", "session_id": "s1", "prompt": "secret", "transcript_path": "/x"}),
            "claude",
        );
        assert_eq!(e["event"], "UserPromptSubmit");
        assert_eq!(e["sessionId"], "s1");
        assert!(e.get("prompt").is_none(), "the prompt is not stored");
        assert!(e.get("transcript_path").is_none());

        let e = ev(
            json!({"hook_event_name": "Stop", "last_assistant_message": "Done."}),
            "claude",
        );
        assert_eq!(e["lastMessage"], "Done.");

        let e = ev(
            json!({"hook_event_name": "StopFailure", "error": "rate_limit"}),
            "claude",
        );
        assert_eq!(e["event"], "StopFailure");
        assert_eq!(e["error"], "rate_limit");

        let e = ev(
            json!({"hook_event_name": "PreToolUse", "tool_name": "ExitPlanMode", "tool_input": {"plan": "Steps:\n1. a", "planFilePath": "/p"}}),
            "claude",
        );
        assert_eq!(e["plan"], "Steps:\n1. a");

        let e = ev(
            json!({"hook_event_name": "PostToolUse", "tool_name": "ExitPlanMode", "tool_input": {"plan": "old"}, "tool_response": {"plan": "approved", "filePath": "/p"}}),
            "claude",
        );
        assert_eq!(e["plan"], "approved");

        let e = ev(
            json!({"hook_event_name": "PostToolUse", "tool_name": "Edit", "tool_input": {"file_path": "C:\\r\\a.txt", "old_string": "x"}}),
            "claude",
        );
        assert_eq!(e["filePath"], "C:\\r\\a.txt");
        assert!(e.get("old_string").is_none());

        let e = ev(
            json!({"hook_event_name": "PostToolUse", "tool_name": "Bash", "tool_input": {"command": "cargo test"}}),
            "claude",
        );
        assert_eq!(e["command"], "cargo test");
    }

    #[test]
    fn gemini_names_map_to_claude_names() {
        let e = ev(
            json!({"hook_event_name": "AfterAgent", "prompt_response": "ok"}),
            "gemini",
        );
        assert_eq!(e["event"], "Stop");
        assert_eq!(e["lastMessage"], "ok");
        let e = ev(json!({"hook_event_name": "BeforeAgent"}), "gemini");
        assert_eq!(e["event"], "UserPromptSubmit");
        // Gemini names mean nothing for other agents.
        assert!(event_from(br#"{"hook_event_name":"AfterAgent"}"#, "claude").is_none());
    }

    #[test]
    fn ignores_other_input() {
        assert!(event_from(b"not json", "claude").is_none());
        assert!(event_from(b"{}", "claude").is_none());
        assert!(event_from(br#"{"hook_event_name":"SessionStart"}"#, "claude").is_none());
        assert!(event_from(br#"{"hook_event_name":5}"#, "claude").is_none());
    }

    #[test]
    fn long_text_is_cut_on_a_char_boundary() {
        let long = "\u{e9}".repeat(MAX_TEXT);
        let e = ev(
            json!({"hook_event_name": "Stop", "last_assistant_message": long}),
            "claude",
        );
        assert!(e["lastMessage"].as_str().unwrap().len() <= MAX_TEXT);
    }

    #[test]
    fn agent_names() {
        assert_eq!(agent_arg(Some("codex")), "codex");
        assert_eq!(agent_arg(Some("../x")), "claude");
        assert_eq!(agent_arg(None), "claude");
    }
}
