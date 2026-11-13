//! Observe-only hooks (docs/agent-terms.md, Phase 3).
//!
//! - The `focuside-hook` helper (crates/hook) ships next to the FocusIDE executable as a
//!   sidecar. It prints nothing, always exits 0 and writes one file per event into the folder
//!   named by `FOCUSIDE_EVENTS`, which every agent pane gets (`<app data>/projects/<id>/
//!   events/<session>`, never the user's repo).
//! - The Claude plugin `focuside` is generated here, in app data, for the current OS: a
//!   manifest, `hooks/hooks.json` in exec form (no shell) and a copy of the helper. Claude panes
//!   get `--plugin-dir <folder>` while the setting "Turn tracking in Claude panes" is on (the
//!   default). Nothing is written to `~/.claude` or to the project.
//! - The plugin has hooks only: no SessionStart, no skills, no MCP, no `asyncRewake`, no
//!   `additionalContext`. The helper sits in `hook/`, not `bin/`, because a plugin's `bin/`
//!   folder goes on the PATH of Claude's Bash tool.
//! - One exception, opt in: with Settings > "Brief replies in Claude panes" on, new panes get a
//!   second generated folder (`focuside-brief`) that also has an output style with
//!   `force-for-plugin: true`. Claude Code applies a forced plugin style while the plugin is
//!   loaded, without the user's `outputStyle` setting (output styles docs, frontmatter
//!   reference). It is text added to Claude's instructions, so it is off by default
//!   (owner decision 2026-11-13). The user's settings files are never written.
//! - Codex and Gemini CLI snippets are shown in Settings for the user to add by hand.
//!   FocusIDE never writes those files.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// File name of the helper on this OS.
pub fn helper_name() -> &'static str {
    if cfg!(windows) {
        "focuside-hook.exe"
    } else {
        "focuside-hook"
    }
}

/// The helper next to the running executable: the installed sidecar, or `target/<profile>`
/// in development (`npm run dev` builds it, see scripts/build-hook.mjs). Tests run from
/// `target/<profile>/deps`, so the parent folder is checked too.
pub fn helper_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let mut dirs = vec![dir.to_path_buf()];
    if dir.file_name().is_some_and(|n| n == "deps") {
        dirs.extend(dir.parent().map(Path::to_path_buf));
    }
    dirs.into_iter()
        .map(|d| d.join(helper_name()))
        .find(|p| p.is_file())
}

/// Where FocusIDE keeps the generated Claude plugin. `brief`: the copy with the brief replies
/// output style. Two folders, so turning the switch on or off changes only panes opened after
/// that; a running pane keeps the folder it started with.
pub fn plugin_dir(data_dir: &Path, brief: bool) -> PathBuf {
    let name = if brief { "focuside-brief" } else { "focuside" };
    data_dir.join("claude-plugin").join(name)
}

/// Tools whose PostToolUse event FocusIDE reads: the plan, the todo list (TodoWrite, or the
/// TaskCreate and TaskUpdate tools that replace it), shell commands and file edits.
const POST_TOOLS: &str = "ExitPlanMode|TodoWrite|TaskCreate|TaskUpdate|Bash|PowerShell|Edit|Write|MultiEdit|NotebookEdit";

/// File name of the brief replies output style in `output-styles/`.
pub const BRIEF_STYLE_FILE: &str = "focuside-brief.md";

/// The brief replies output style. `keep-coding-instructions` keeps Claude Code's software
/// engineering instructions (a custom style drops them otherwise); `force-for-plugin` turns
/// the style on while the plugin is loaded, with no change to the user's settings.
pub const BRIEF_STYLE: &str = "\
---
name: FocusIDE brief replies
description: Short replies for Claude panes in FocusIDE. Plans and todo lists show in the FocusIDE plan panel.
keep-coding-instructions: true
force-for-plugin: true
---

You run in a terminal pane inside FocusIDE. FocusIDE shows your plans and todo lists in a plan panel next to this terminal, and it shows the changed files of each prompt.

- Answer in one short line.
- When you create or change a plan or a todo list, say only: \"Plan updated. See the plan panel.\"
- When you finish work, say in one line what changed.
- Do not write long explanations unless the user asks for them.
";

/// `hooks/hooks.json`: the event map wrapped in a top-level "hooks" key. Exec form (`args`
/// present): Claude Code spawns the helper directly, no shell and no quoting on any OS, and
/// substitutes `${CLAUDE_PLUGIN_ROOT}`. On Windows exec form needs a real `.exe`.
pub fn hooks_json() -> Value {
    let handler = json!([{
        "type": "command",
        "command": format!("${{CLAUDE_PLUGIN_ROOT}}/hook/{}", helper_name()),
        "args": ["claude"],
        "timeout": 10
    }]);
    json!({
        "description": "FocusIDE turn tracking. Observe-only: writes when a turn starts and ends to a FocusIDE folder. Prints nothing.",
        "hooks": {
            "UserPromptSubmit": [{ "hooks": handler }],
            "Stop": [{ "hooks": handler }],
            "StopFailure": [{ "hooks": handler }],
            "PreToolUse": [{ "matcher": "ExitPlanMode", "hooks": handler }],
            "PostToolUse": [{ "matcher": POST_TOOLS, "hooks": handler }]
        }
    })
}

/// `.claude-plugin/plugin.json`. The name must not start with `claude-` or `anthropic-` and
/// should not contain "claude" as a word; `focuside` does neither.
pub fn manifest(brief: bool) -> Value {
    let description = if brief {
        "FocusIDE turn tracking for Claude panes in FocusIDE, with the brief replies output style the user turned on."
    } else {
        "FocusIDE turn tracking for Claude panes in FocusIDE. Observe-only hooks that print nothing."
    };
    json!({
        "name": "focuside",
        "version": env!("CARGO_PKG_VERSION"),
        "description": description,
        "author": { "name": "FocusIDE" },
        "homepage": "https://github.com/yobotn/focuside",
        "license": "GPL-3.0-only"
    })
}

/// Writes `bytes` to `path` unless it already holds them.
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if std::fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).with_context(|| format!("write {}", path.display()))
}

/// Writes the plugin folder for this OS and copies the helper into it. Returns the folder.
/// When the helper copy is in use (a hook runs right now), the old copy stays. `brief` writes
/// the `focuside-brief` folder, which also has the brief replies output style.
pub fn ensure_plugin(data_dir: &Path, helper: &Path, brief: bool) -> Result<PathBuf> {
    let dir = plugin_dir(data_dir, brief);
    write_if_changed(
        &dir.join(".claude-plugin").join("plugin.json"),
        &serde_json::to_vec_pretty(&manifest(brief))?,
    )?;
    write_if_changed(
        &dir.join("hooks").join("hooks.json"),
        &serde_json::to_vec_pretty(&hooks_json())?,
    )?;
    if brief {
        write_if_changed(
            &dir.join("output-styles").join(BRIEF_STYLE_FILE),
            BRIEF_STYLE.as_bytes(),
        )?;
    }
    let bytes = std::fs::read(helper).with_context(|| format!("read {}", helper.display()))?;
    let dest = dir.join("hook").join(helper_name());
    if let Err(e) = write_if_changed(&dest, &bytes) {
        if !dest.is_file() {
            return Err(e);
        }
        tracing::warn!("hook helper copy: {e:#}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(dir)
}

/// A path in single quotes for POSIX shells.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Codex `~/.codex/hooks.json`. Codex runs `command` in a shell; on Windows `commandWindows`
/// replaces it (run through `cmd /C`). The helper prints nothing, so UserPromptSubmit adds no
/// developer context and Stop gets no output to parse.
pub fn codex_snippet(helper: &str, windows: bool) -> String {
    let mut handler = json!({ "type": "command", "timeout": 10 });
    if windows {
        let cmd = format!("\"{helper}\" codex");
        handler["command"] = json!(cmd);
        handler["commandWindows"] = json!(cmd);
    } else {
        handler["command"] = json!(format!("{} codex", sh_quote(helper)));
    }
    let h = json!([handler]);
    let v = json!({
        "hooks": {
            "UserPromptSubmit": [{ "hooks": h }],
            "Stop": [{ "hooks": h }],
            "PreToolUse": [{ "matcher": "*", "hooks": h }],
            "PostToolUse": [{ "matcher": "*", "hooks": h }]
        }
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Gemini CLI `~/.gemini/settings.json`, the `hooks` key. Gemini runs the command in
/// PowerShell on Windows (so the call operator `&`) and in bash elsewhere. Timeouts are in
/// milliseconds.
pub fn gemini_snippet(helper: &str, windows: bool) -> String {
    let command = if windows {
        format!("& '{}' gemini", helper.replace('\'', "''"))
    } else {
        format!("{} gemini", sh_quote(helper))
    };
    let h =
        json!([{ "name": "focuside", "type": "command", "command": command, "timeout": 10000 }]);
    let v = json!({
        "hooks": {
            "BeforeAgent": [{ "matcher": "*", "hooks": h }],
            "AfterAgent": [{ "matcher": "*", "hooks": h }],
            "BeforeTool": [{ "matcher": "*", "hooks": h }],
            "AfterTool": [{ "matcher": "*", "hooks": h }]
        }
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_is_observe_only() {
        let h = hooks_json();
        let events: Vec<&String> = h["hooks"].as_object().unwrap().keys().collect();
        for e in &events {
            assert!(
                [
                    "UserPromptSubmit",
                    "Stop",
                    "StopFailure",
                    "PreToolUse",
                    "PostToolUse"
                ]
                .contains(&e.as_str()),
                "unexpected event {e}"
            );
        }
        let text = h.to_string();
        for banned in [
            "SessionStart",
            "UserPromptExpansion",
            "asyncRewake",
            "additionalContext",
            "\"async\"",
        ] {
            assert!(!text.contains(banned), "hooks.json contains {banned}");
        }
        for (_, groups) in h["hooks"].as_object().unwrap() {
            for g in groups.as_array().unwrap() {
                for handler in g["hooks"].as_array().unwrap() {
                    assert!(handler["args"].is_array(), "exec form");
                    assert!(handler["command"]
                        .as_str()
                        .unwrap()
                        .starts_with("${CLAUDE_PLUGIN_ROOT}/hook/"));
                }
            }
        }
        let name = manifest(false)["name"].as_str().unwrap().to_string();
        assert_eq!(name, "focuside");
        assert!(!name.starts_with("claude") && !name.starts_with("anthropic"));
    }

    #[test]
    fn writes_the_plugin_folder() {
        let data = tempfile::tempdir().unwrap();
        let helper = data.path().join("helper-src");
        std::fs::write(&helper, b"binary").unwrap();
        let dir = ensure_plugin(data.path(), &helper, false).unwrap();
        assert!(dir.join(".claude-plugin/plugin.json").is_file());
        let hooks: Value =
            serde_json::from_slice(&std::fs::read(dir.join("hooks/hooks.json")).unwrap()).unwrap();
        assert!(hooks["hooks"]["Stop"].is_array());
        assert_eq!(
            std::fs::read(dir.join("hook").join(helper_name())).unwrap(),
            b"binary"
        );
        assert!(!dir.join("bin").exists(), "bin/ would go on Claude's PATH");
        assert!(
            !dir.join("output-styles").exists(),
            "no output style unless brief replies is on"
        );
        // A second call changes nothing and still works.
        ensure_plugin(data.path(), &helper, false).unwrap();

        // Brief replies: a separate folder with the same hooks and the forced output style.
        let brief = ensure_plugin(data.path(), &helper, true).unwrap();
        assert_ne!(brief, dir);
        let style =
            std::fs::read_to_string(brief.join("output-styles").join(BRIEF_STYLE_FILE)).unwrap();
        assert!(style.starts_with("---\n"));
        assert!(style.contains("force-for-plugin: true"));
        assert!(style.contains("keep-coding-instructions: true"));
        assert!(brief.join("hooks/hooks.json").is_file());
    }

    /// Live check with the installed Claude Code and the built helper (run
    /// `node scripts/build-hook.mjs` first):
    /// `cargo test -p focuside-desktop -- --ignored real_claude_loads_the_plugin`.
    /// Generates the plugin, runs `claude plugin validate --strict` and `plugin details` with
    /// `--plugin-dir`, then feeds the plugin's helper copy a Stop event. No prompt is sent.
    #[test]
    #[ignore]
    fn real_claude_loads_the_plugin() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let helper = helper_path().expect("focuside-hook built next to the test binary");
        let data = tempfile::tempdir().unwrap();
        let dir = ensure_plugin(data.path(), &helper, false).unwrap();
        let brief = ensure_plugin(data.path(), &helper, true).unwrap();
        let claude = focuside_core::agent::locate_claude().expect("claude installed");
        let run = |args: &[&str]| {
            let out = Command::new(&claude)
                .args(args)
                .current_dir(data.path())
                .output()
                .expect("run claude");
            let text = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            println!("claude {args:?}\n{text}");
            (out.status.success(), text)
        };
        let d = dir.to_string_lossy().to_string();
        let (ok, _) = run(&["plugin", "validate", &d, "--strict"]);
        assert!(ok, "plugin validate --strict");
        let (ok, text) = run(&["--plugin-dir", &d, "plugin", "details", "focuside@inline"]);
        assert!(ok);
        assert!(text.contains("Hooks (5)"), "{text}");
        assert!(text.contains("Skills (0)") && text.contains("MCP servers (0)"));

        // The brief replies copy: valid, same hooks, plus the output style.
        let b = brief.to_string_lossy().to_string();
        let (ok, _) = run(&["plugin", "validate", &b, "--strict"]);
        assert!(ok, "plugin validate --strict (brief)");
        let (ok, text) = run(&["--plugin-dir", &b, "plugin", "details", "focuside@inline"]);
        assert!(ok);
        assert!(text.contains("Hooks (5)"), "{text}");

        // The copy in the plugin folder writes an event and prints nothing.
        let events = data.path().join("events");
        let mut child = Command::new(dir.join("hook").join(helper_name()))
            .arg("claude")
            .env("FOCUSIDE_EVENTS", &events)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                br#"{"hook_event_name":"Stop","session_id":"x","last_assistant_message":"Done."}"#,
            )
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
        let evs = crate::pane::take_events(&events);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0]["event"], "Stop");
        assert_eq!(evs[0]["lastMessage"], "Done.");
    }

    #[test]
    fn snippets_parse_and_quote_paths() {
        let win = r"C:\Program Files\FocusIDE\focuside-hook.exe";
        let c: Value = serde_json::from_str(&codex_snippet(win, true)).unwrap();
        let handler = &c["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(handler["commandWindows"], format!("\"{win}\" codex"));
        let g: Value = serde_json::from_str(&gemini_snippet(r"C:\it's\x.exe", true)).unwrap();
        assert_eq!(
            g["hooks"]["AfterAgent"][0]["hooks"][0]["command"],
            r"& 'C:\it''s\x.exe' gemini"
        );
        let g: Value = serde_json::from_str(&gemini_snippet("/opt/my app/h", false)).unwrap();
        assert_eq!(
            g["hooks"]["BeforeTool"][0]["hooks"][0]["command"],
            "'/opt/my app/h' gemini"
        );
        let c: Value = serde_json::from_str(&codex_snippet("/a/h", false)).unwrap();
        assert!(c["hooks"]["Stop"][0]["hooks"][0]
            .get("commandWindows")
            .is_none());
    }
}
