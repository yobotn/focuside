//! Runs the built helper the way an agent does and checks the hard rules: no stdout, no
//! stderr, exit code 0, for every event type and for bad input, with and without
//! FOCUSIDE_EVENTS.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(events: Option<&Path>, args: &[&str], input: &[u8]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_focuside-hook"));
    cmd.args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Windows needs SystemRoot for some process start paths.
    if let Some(root) = std::env::var_os("SystemRoot") {
        cmd.env("SystemRoot", root);
    }
    if let Some(dir) = events {
        cmd.env("FOCUSIDE_EVENTS", dir);
    }
    let mut child = cmd.spawn().expect("spawn helper");
    // The helper may exit before reading (no FOCUSIDE_EVENTS): ignore a broken pipe.
    let _ = child.stdin.take().unwrap().write_all(input);
    child.wait_with_output().expect("helper output")
}

fn assert_silent(out: &std::process::Output, what: &str) {
    assert_eq!(out.status.code(), Some(0), "{what}: exit code");
    assert!(out.stdout.is_empty(), "{what}: stdout {:?}", out.stdout);
    assert!(out.stderr.is_empty(), "{what}: stderr {:?}", out.stderr);
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

const EVENTS: &[(&str, &str)] = &[
    (
        "claude",
        r#"{"hook_event_name":"UserPromptSubmit","session_id":"s","prompt":"hi"}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"Stop","session_id":"s","last_assistant_message":"Done"}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"StopFailure","session_id":"s","error":"rate_limit"}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"PreToolUse","tool_name":"ExitPlanMode","tool_input":{"plan":"1. a"}}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"PostToolUse","tool_name":"ExitPlanMode","tool_input":{},"tool_response":{"plan":"1. a","filePath":"p"}}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"PostToolUse","tool_name":"Bash","tool_input":{"command":"ls"}}"#,
    ),
    (
        "claude",
        r#"{"hook_event_name":"PostToolUse","tool_name":"Write","tool_input":{"file_path":"/r/a"}}"#,
    ),
    (
        "codex",
        r#"{"hook_event_name":"Stop","turn_id":"t","last_assistant_message":"ok"}"#,
    ),
    (
        "gemini",
        r#"{"hook_event_name":"BeforeAgent","prompt":"hi"}"#,
    ),
    (
        "gemini",
        r#"{"hook_event_name":"AfterAgent","prompt_response":"ok"}"#,
    ),
    (
        "gemini",
        r#"{"hook_event_name":"AfterTool","tool_name":"run_shell_command","tool_input":{"command":"ls"}}"#,
    ),
];

#[test]
fn every_event_is_silent_and_written() {
    let dir = tempfile::tempdir().unwrap();
    for (i, (agent, input)) in EVENTS.iter().enumerate() {
        let out = run(Some(dir.path()), &[agent], input.as_bytes());
        assert_silent(&out, input);
        let names = files(dir.path());
        assert_eq!(names.len(), i + 1, "one file per event: {names:?}");
        assert!(names.iter().all(|n| n.ends_with(".json")), "{names:?}");
    }
    // Each file is valid JSON with the schema version.
    for n in files(dir.path()) {
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join(n)).unwrap()).unwrap();
        assert_eq!(v["v"], 1);
    }
}

#[test]
fn bad_input_is_silent_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let bad: &[&[u8]] = &[
        b"",
        b"not json",
        b"{",
        b"[]",
        b"{}",
        br#"{"hook_event_name":"SessionStart"}"#,
        br#"{"hook_event_name":"UserPromptExpansion"}"#,
        &[0xff, 0xfe, 0x00],
    ];
    for input in bad {
        let out = run(Some(dir.path()), &["claude"], input);
        assert_silent(&out, &String::from_utf8_lossy(input));
    }
    let out = run(Some(dir.path()), &["../../evil"], EVENTS[0].1.as_bytes());
    assert_silent(&out, "bad agent argument");
    assert_eq!(
        files(dir.path()).len(),
        1,
        "a bad agent name falls back to claude"
    );
}

#[test]
fn missing_variable_is_silent() {
    for (agent, input) in EVENTS {
        let out = run(None, &[agent], input.as_bytes());
        assert_silent(&out, input);
    }
}

#[test]
fn unwritable_folder_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    // A file where the folder should be: create_dir_all fails.
    let file = dir.path().join("events");
    std::fs::write(&file, "x").unwrap();
    for (agent, input) in EVENTS {
        let out = run(Some(&file), &[agent], input.as_bytes());
        assert_silent(&out, input);
    }
}
