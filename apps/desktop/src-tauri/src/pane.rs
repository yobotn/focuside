//! A pane session: the agent's own unmodified TUI (for example `claude`) in a PTY, shown in
//! the center view. The user types every prompt there. FocusIDE only records turns around it:
//! snapshots, changes and undo.
//!
//! Every pane gets `FOCUSIDE_EVENTS`, a folder in app data where the `focuside-hook` helper
//! writes hook events (see hooks.rs). A watcher on that folder feeds `on_hook`.
//!
//! Turns come from hooks only: a turn starts on UserPromptSubmit and ends on Stop or
//! StopFailure. The claim is held only in between. It is also released by the next
//! UserPromptSubmit (Stop does not fire after an interrupt), pane exit and pane close. A prompt
//! that arrives while another session holds the claim records no turn. A pane without hooks
//! (turn tracking off, hooks blocked, or an agent without hooks) records no turns.
//!
//! Hook events also carry the plan (ExitPlanMode), the todo list (TodoWrite, TaskCreate,
//! TaskUpdate) and Claude's permission mode. The mode button sends one Shift+Tab key
//! sequence (`MODE_KEY`) on a user click, only while no prompt runs (see `mode_key_ready`).
//!
//! Events go to the UI as `session` events, like chat sessions: `turn_started`, `turn_ended`,
//! `turn_untracked`, `pane_started`, `pane_exited`, `pane_prompt`, `pane_working`, `pane_idle`,
//! `pane_plan`, `pane_todos`, `pane_mode`, `hooks_live`.
//!
//! `pane_prompt` carries the prompt text from UserPromptSubmit and `pane_idle` the last answer
//! from Stop, for the Prompts panel. The UI keeps them in FocusIDE's app data only.

use crate::project::Project;
use crate::pty::Launch;
use crate::turn_recorder::{new_id, TurnEnd, TurnRecorder};
use focuside_core::agent::TuiCommand;
use focuside_core::plan;
use focuside_core::snapshot::Snapshot;
use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// The key sequence of Shift+Tab (CSI Z). The mode button sends it; Claude Code moves to its
/// next permission mode. It is the same input as the user pressing Shift+Tab in the pane.
pub const MODE_KEY: &[u8] = b"\x1b[Z";

struct PaneTurn {
    id: String,
    start: Snapshot,
    your_edits: BTreeSet<String>,
    /// From PostToolUse: files the agent's edit tools wrote, shell commands it ran.
    edits: BTreeSet<String>,
    commands: Vec<String>,
    plan_steps: Vec<String>,
    last_message: Option<String>,
    failure: Option<String>,
}

#[derive(Default)]
struct PaneState {
    /// Id of the PTY that runs the TUI now. None before it opens and after it exits.
    pty: Option<String>,
    turn: Option<PaneTurn>,
    closed: bool,
    /// A hook event arrived from this pane.
    hooks_live: bool,
    /// A prompt runs: between UserPromptSubmit and Stop or StopFailure.
    working: bool,
    /// The permission mode from the last hook event that had one.
    mode: Option<String>,
    /// The agent's todo list, from TodoWrite or the Task tools.
    todos: Vec<Todo>,
}

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    /// Task id (TaskCreate). None for TodoWrite items.
    id: Option<String>,
    content: String,
    status: String,
}

/// Applies one todo change from the helper (`todo` field, see crates/hook) to `list`.
fn apply_todo(list: &mut Vec<Todo>, change: &Value) {
    let s = |v: &Value| v.as_str().map(String::from);
    match change["kind"].as_str() {
        Some("list") => {
            *list = change["items"]
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or_default()
                .iter()
                .filter_map(|t| {
                    Some(Todo {
                        id: None,
                        content: s(&t["content"])?,
                        status: s(&t["status"]).unwrap_or_else(|| "pending".into()),
                    })
                })
                .collect();
        }
        Some("create") => {
            if let Some(content) = s(&change["subject"]) {
                // A list from TodoWrite does not mix with Task tools.
                list.retain(|t| t.id.is_some());
                list.push(Todo {
                    id: s(&change["id"]),
                    content,
                    status: "pending".into(),
                });
            }
        }
        Some("update") => {
            let id = s(&change["id"]);
            let Some(i) = list.iter().position(|t| t.id.is_some() && t.id == id) else {
                return;
            };
            match change["status"].as_str() {
                Some("deleted") => {
                    list.remove(i);
                }
                Some(st) => list[i].status = st.to_string(),
                None => {}
            }
            if let (Some(t), Some(c)) = (list.get_mut(i), s(&change["subject"])) {
                t.content = c;
            }
        }
        _ => {}
    }
}

/// Sends a `session` event to the UI.
type Sink = Arc<dyn Fn(Value) + Send + Sync>;

pub struct Pane {
    pub key: String,
    pub project: Arc<Project>,
    /// Agent id ("claude", "codex", ...). Events from another agent's hooks are ignored.
    pub agent: String,
    /// What `pty_open` starts for this pane. Resolved in Rust, never sent by the UI.
    pub launch: Launch,
    /// `FOCUSIDE_EVENTS` for this pane.
    pub events_dir: PathBuf,
    /// Started with the focuside plugin (`--plugin-dir`), so hooks are expected.
    pub hooks_expected: bool,
    /// Claude only: the mode button may send Shift+Tab.
    pub mode_key: bool,
    sink: Sink,
    recorder: TurnRecorder,
    st: Mutex<PaneState>,
    /// Turn start and end never run at the same time.
    turn_lock: tokio::sync::Mutex<()>,
    /// Keeps the events folder watcher alive.
    watcher: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

/// The PTY launch for an agent TUI: the resolved program and arguments, plus
/// `FOCUSIDE_EVENTS`. With `plugin`, `--plugin-dir <plugin>` is added (Claude only, while turn
/// tracking is on). Nothing else is added and nothing is removed.
pub fn launch_for(tui: TuiCommand, events_dir: &Path, plugin: Option<&Path>) -> Launch {
    let mut args = tui.args;
    if let Some(p) = plugin {
        args.push("--plugin-dir".into());
        args.push(p.to_string_lossy().to_string());
    }
    Launch {
        program: tui.program.to_string_lossy().to_string(),
        args,
        env: vec![(
            "FOCUSIDE_EVENTS".into(),
            events_dir.to_string_lossy().to_string(),
        )],
    }
}

/// The events folder of a pane session, in app data.
pub fn events_dir(project: &Project, key: &str) -> PathBuf {
    project.data_dir.join("events").join(key)
}

/// Reads, removes and returns the event files in `dir`, oldest first. Files that do not parse
/// are removed too.
pub fn take_events(dir: &Path) -> Vec<Value> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut names: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for p in names {
        let bytes = std::fs::read(&p);
        let _ = std::fs::remove_file(&p);
        let parsed = bytes
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        if let Some(v) = parsed.filter(Value::is_object) {
            out.push(v);
        }
    }
    out
}

impl Pane {
    pub fn new(
        app: AppHandle,
        project: Arc<Project>,
        agent: &str,
        tui: TuiCommand,
        plugin: Option<PathBuf>,
    ) -> Arc<Self> {
        let sink: Sink = Arc::new(move |data| {
            let _ = app.emit("session", data);
        });
        let pane = Self::with_sink(sink, project, agent, tui, plugin);
        pane.watch_events();
        pane
    }

    /// A pane that sends its events to `sink`. `new` uses the app; tests use a list.
    fn with_sink(
        sink: Sink,
        project: Arc<Project>,
        agent: &str,
        tui: TuiCommand,
        plugin: Option<PathBuf>,
    ) -> Arc<Self> {
        let key = new_id("s");
        let events_dir = events_dir(&project, &key);
        // A fresh folder: no events from an earlier run.
        let _ = std::fs::remove_dir_all(&events_dir);
        if let Err(e) = std::fs::create_dir_all(&events_dir) {
            tracing::warn!("events folder {}: {e}", events_dir.display());
        }
        Arc::new(Self {
            recorder: TurnRecorder::new(project.clone(), &key),
            launch: launch_for(tui, &events_dir, plugin.as_deref()),
            hooks_expected: plugin.is_some(),
            mode_key: agent == "claude",
            key,
            project,
            agent: agent.to_string(),
            events_dir,
            sink,
            st: Mutex::new(PaneState::default()),
            turn_lock: tokio::sync::Mutex::new(()),
            watcher: Mutex::new(None),
        })
    }

    fn emit(&self, kind: &str, mut data: Value) {
        if !data.is_object() {
            data = json!({});
        }
        data["key"] = json!(self.key);
        data["kind"] = json!(kind);
        data["projectId"] = json!(self.project.id);
        if cfg!(debug_assertions) {
            eprintln!("[event] {} {}", self.key, kind);
        }
        (self.sink)(data);
    }

    /// Watches the events folder. Events are handled one at a time, in file order.
    fn watch_events(self: &Arc<Self>) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
        let dir = self.events_dir.clone();
        let tx2 = tx.clone();
        let debouncer = new_debouncer(Duration::from_millis(50), move |_: DebounceEventResult| {
            for ev in take_events(&dir) {
                let _ = tx2.send(ev);
            }
        });
        match debouncer {
            Ok(mut d) => {
                if let Err(e) = d
                    .watcher()
                    .watch(&self.events_dir, RecursiveMode::NonRecursive)
                {
                    tracing::warn!("watch {}: {e}", self.events_dir.display());
                }
                *self.watcher.lock().unwrap() = Some(Box::new(d));
            }
            Err(e) => tracing::warn!("events watcher: {e}"),
        }
        for ev in take_events(&self.events_dir) {
            let _ = tx.send(ev);
        }
        // The loop ends when the watcher (the last sender) is dropped on close.
        drop(tx);
        let weak = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let Some(p) = weak.upgrade() else { break };
                p.on_hook(ev).await;
            }
        });
    }

    /// Sets the PTY that runs the TUI. Exit events of other PTYs are ignored from then on.
    pub fn set_pty(&self, id: Option<String>) {
        self.st.lock().unwrap().pty = id;
    }

    fn running(&self) -> bool {
        let st = self.st.lock().unwrap();
        !st.closed && st.pty.is_some()
    }

    /// The TUI started: close a turn left open. The next prompt starts a turn (hooks).
    pub async fn on_pty_open(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.end_locked("open").await;
        self.st.lock().unwrap().working = false;
        if self.running() {
            self.emit(
                "pane_started",
                json!({ "hooksExpected": self.hooks_expected }),
            );
            self.emit("pane_idle", json!({}));
        }
    }

    /// The TUI exited (or was killed). Ends the open turn and releases the claim.
    pub async fn on_pty_exit(self: &Arc<Self>, pty_id: &str) {
        {
            let mut st = self.st.lock().unwrap();
            if st.pty.as_deref() != Some(pty_id) {
                return;
            }
            st.pty = None;
            st.working = false;
        }
        let _g = self.turn_lock.lock().await;
        self.end_locked("exit").await;
        self.emit("pane_exited", json!({}));
    }

    /// One hook event from the events folder (see crates/hook for the fields).
    pub async fn on_hook(self: &Arc<Self>, ev: Value) {
        if ev["agent"].as_str() != Some(self.agent.as_str()) {
            return;
        }
        let event = ev["event"].as_str().unwrap_or("");
        let _g = self.turn_lock.lock().await;
        let (first, mode) = {
            let mut st = self.st.lock().unwrap();
            if st.closed {
                return;
            }
            let first = !std::mem::replace(&mut st.hooks_live, true);
            let mode = ev["mode"]
                .as_str()
                .filter(|m| st.mode.as_deref() != Some(*m))
                .map(String::from);
            if mode.is_some() {
                st.mode.clone_from(&mode);
            }
            (first, mode)
        };
        if first {
            self.emit("hooks_live", json!({}));
        }
        if let Some(m) = mode {
            self.emit("pane_mode", json!({ "mode": m }));
        }
        match event {
            "UserPromptSubmit" => {
                self.emit("pane_prompt", json!({ "text": ev["prompt"].as_str() }));
                self.st.lock().unwrap().working = true;
                self.emit("pane_working", json!({}));
                // Also closes a turn whose Stop never came (interrupt).
                self.end_locked("prompt").await;
                self.begin_locked().await;
            }
            "Stop" | "StopFailure" => {
                let failure = ev["error"].as_str().map(String::from);
                let last = ev["lastMessage"].as_str().map(String::from);
                {
                    let mut st = self.st.lock().unwrap();
                    st.working = false;
                    if let Some(t) = st.turn.as_mut() {
                        if event == "Stop" {
                            t.last_message = ev["lastMessage"].as_str().map(String::from);
                        } else {
                            t.failure = failure.clone().or(Some(String::new()));
                        }
                    }
                }
                let reason = if event == "Stop" {
                    "stop"
                } else {
                    "stopFailure"
                };
                self.end_locked(reason).await;
                self.emit(
                    "pane_idle",
                    json!({ "failure": failure, "lastMessage": last }),
                );
            }
            "PreToolUse" | "PostToolUse" => self.on_tool(event, &ev),
            _ => {}
        }
    }

    fn on_tool(&self, event: &str, ev: &Value) {
        if ev["tool"] == "ExitPlanMode" {
            if let Some(text) = ev["plan"].as_str().filter(|s| !s.trim().is_empty()) {
                let mut parsed = plan::parse(text);
                plan::check_paths(&mut parsed, &self.project.root);
                let turn_id = {
                    let mut st = self.st.lock().unwrap();
                    st.turn.as_mut().map(|t| {
                        t.plan_steps = parsed.steps.clone();
                        t.id.clone()
                    })
                };
                self.emit(
                    "pane_plan",
                    json!({
                        "turnId": turn_id,
                        "planText": text,
                        "plan": parsed,
                        "approved": event == "PostToolUse",
                    }),
                );
            }
        }
        if event != "PostToolUse" {
            return;
        }
        if ev["todo"].is_object() {
            let todos: Vec<Value> = {
                let mut st = self.st.lock().unwrap();
                apply_todo(&mut st.todos, &ev["todo"]);
                st.todos
                    .iter()
                    .map(|t| json!({ "content": t.content, "status": t.status }))
                    .collect()
            };
            self.emit("pane_todos", json!({ "todos": todos }));
        }
        let mut st = self.st.lock().unwrap();
        let Some(t) = st.turn.as_mut() else { return };
        if let Some(c) = ev["command"].as_str().filter(|c| !c.trim().is_empty()) {
            t.commands.push(c.to_string());
        }
        if let Some(p) = ev["filePath"].as_str() {
            if let Some(rel) = crate::session::inside(&self.project.root, p) {
                t.edits.insert(rel);
            }
        }
    }

    /// The mode button may send Shift+Tab: a Claude TUI runs and no prompt runs. While a prompt
    /// runs, Claude can show a permission or plan approval question, and Shift+Tab there
    /// answers it ("pressing Shift+Tab there approves the plan", permission modes docs). Err
    /// holds the reason for the UI.
    pub fn mode_key_ready(&self) -> Result<String, &'static str> {
        let st = self.st.lock().unwrap();
        if !self.mode_key {
            return Err("noModeKey");
        }
        if st.closed {
            return Err("closed");
        }
        if st.working {
            return Err("busy");
        }
        st.pty.clone().ok_or("closed")
    }

    async fn begin_locked(self: &Arc<Self>) {
        if !self.running() || self.st.lock().unwrap().turn.is_some() {
            return;
        }
        if !self.recorder.claim() {
            // The session that holds the project, so the UI can name it.
            let holder = self.project.running_session();
            self.emit(
                "turn_untracked",
                json!({ "reason": "otherRunning", "holder": holder }),
            );
            return;
        }
        let id = new_id("t");
        match self.recorder.start(&id).await {
            Ok(start) => {
                self.st.lock().unwrap().turn = Some(PaneTurn {
                    id: id.clone(),
                    start,
                    your_edits: BTreeSet::new(),
                    edits: BTreeSet::new(),
                    commands: vec![],
                    plan_steps: vec![],
                    last_message: None,
                    failure: None,
                });
                self.emit("turn_started", json!({ "turnId": id, "pane": true }));
            }
            Err(e) => self.emit(
                "turn_untracked",
                json!({ "reason": "snapshot", "error": format!("{e:#}") }),
            ),
        }
    }

    async fn end_locked(self: &Arc<Self>, reason: &str) -> bool {
        let Some(turn) = self.st.lock().unwrap().turn.take() else {
            return false;
        };
        let mut data = json!({
            "turnId": turn.id,
            "stopReason": "end_turn",
            "error": null,
            "hasSnapshot": true,
            "pane": true,
            "reason": reason,
            "failure": turn.failure,
        });
        let res = self
            .recorder
            .finish(TurnEnd {
                id: &turn.id,
                start: &turn.start,
                edits: &turn.edits,
                commands: turn.commands.clone(),
                your_edits: &turn.your_edits,
                continued: false,
                // Hook turns report edits and commands (PostToolUse), like ACP tool calls.
                tools_known: true,
                plan_steps: &turn.plan_steps,
                last_message: turn.last_message.as_deref(),
            })
            .await;
        match res {
            Ok(r) => r.write_to(&mut data),
            Err(e) => data["snapshotError"] = json!(format!("{e:#}")),
        }
        self.emit("turn_ended", data);
        true
    }

    /// Closes the pane session: ends the open turn, releases the claim, stops the events
    /// watcher and removes the events folder. The caller kills the PTY.
    pub async fn close(self: &Arc<Self>) {
        {
            let mut st = self.st.lock().unwrap();
            st.closed = true;
            st.pty = None;
        }
        let _g = self.turn_lock.lock().await;
        self.end_locked("close").await;
        self.recorder.release();
        self.watcher.lock().unwrap().take();
        let _ = std::fs::remove_dir_all(&self.events_dir);
    }

    /// True while this pane holds the project for an open turn.
    pub fn holds_claim(&self) -> bool {
        self.recorder.holds_claim()
    }

    /// Records a save from the FocusIDE viewer during the open turn ("your edit").
    pub fn note_user_save(&self, rel: &str) {
        if let Some(t) = self.st.lock().unwrap().turn.as_mut() {
            t.your_edits.insert(rel.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn_recorder::TurnRecorder;

    struct Fixture {
        root: tempfile::TempDir,
        _data: tempfile::TempDir,
        project: Arc<Project>,
        events: Arc<Mutex<Vec<Value>>>,
    }

    async fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.txt"), "one\n").unwrap();
        let git = focuside_core::which::find("git").expect("git on PATH");
        let project = Project::open(root.path(), data.path(), &git).await.unwrap();
        Fixture {
            root,
            _data: data,
            project,
            events: Arc::new(Mutex::new(vec![])),
        }
    }

    fn pane(f: &Fixture, plugin: Option<PathBuf>) -> Arc<Pane> {
        let events = f.events.clone();
        let sink: Sink = Arc::new(move |v| events.lock().unwrap().push(v));
        let tui = TuiCommand {
            program: PathBuf::from("claude"),
            args: vec![],
        };
        let p = Pane::with_sink(sink, f.project.clone(), "claude", tui, plugin);
        p.set_pty(Some("pty1".into()));
        p
    }

    fn kinds(f: &Fixture) -> Vec<String> {
        f.events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e["kind"].as_str().unwrap_or("").to_string())
            .collect()
    }

    fn hook(event: &str, extra: Value) -> Value {
        let mut v = json!({ "v": 1, "agent": "claude", "event": event });
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }

    #[test]
    fn launch_adds_only_the_events_folder_and_the_plugin() {
        let tui = TuiCommand {
            program: PathBuf::from("claude"),
            args: vec![],
        };
        let l = launch_for(tui.clone(), Path::new("/d/ev"), None);
        assert!(l.args.is_empty());
        assert_eq!(l.env.len(), 1);
        assert_eq!(l.env[0].0, "FOCUSIDE_EVENTS");
        let l = launch_for(tui, Path::new("/d/ev"), Some(Path::new("/d/plugin")));
        assert_eq!(l.args, vec!["--plugin-dir", "/d/plugin"]);
    }

    #[tokio::test]
    async fn hook_turns_hold_the_claim_only_while_a_prompt_runs() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        let chat = TurnRecorder::new(f.project.clone(), "chat");

        // Expected hooks: opening the pane starts no turn and holds no claim.
        p.on_pty_open().await;
        assert!(!p.holds_claim());
        assert!(chat.claim(), "a chat turn can run while the pane waits");
        chat.release();

        // UserPromptSubmit starts a turn; PostToolUse adds edits and commands; Stop ends it.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        assert!(p.holds_claim());
        assert!(!chat.claim(), "the pane's prompt holds the project");
        std::fs::write(f.root.path().join("a.txt"), "two\n").unwrap();
        let abs = f.root.path().join("a.txt").to_string_lossy().to_string();
        p.on_hook(hook(
            "PostToolUse",
            json!({ "tool": "Edit", "filePath": abs }),
        ))
        .await;
        p.on_hook(hook(
            "PostToolUse",
            json!({ "tool": "Bash", "command": "cargo test" }),
        ))
        .await;
        p.on_hook(hook(
            "PreToolUse",
            json!({ "tool": "ExitPlanMode", "plan": "Steps:\n1. Edit a.txt\n2. Test" }),
        ))
        .await;
        p.on_hook(hook("Stop", json!({ "lastMessage": "Changed a.txt." })))
            .await;
        let idle = f
            .events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|e| e["kind"] == "pane_idle")
            .cloned()
            .unwrap();
        assert_eq!(idle["lastMessage"], "Changed a.txt.");
        assert!(!p.holds_claim(), "Stop releases the claim");
        assert!(chat.claim());
        chat.release();

        let evs = f.events.lock().unwrap().clone();
        let ended = evs.iter().find(|e| e["kind"] == "turn_ended").unwrap();
        assert_eq!(ended["summary"]["files"], 1);
        assert_eq!(ended["summary"]["commands"][0], "cargo test");
        assert_eq!(ended["summary"]["planStepCount"], 2);
        assert_eq!(ended["summary"]["excerpt"], "Changed a.txt.");
        assert_eq!(ended["outside"], false);
        assert!(evs.iter().any(|e| e["kind"] == "pane_plan"));
        let turns = f.project.turns.lock().unwrap().clone();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].commands, vec!["cargo test"]);
        assert_eq!(turns[0].edits, vec!["a.txt"]);
    }

    #[tokio::test]
    async fn prompt_while_another_session_runs_is_not_tracked() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        p.on_pty_open().await;
        let chat = TurnRecorder::new(f.project.clone(), "chat");
        assert!(chat.claim());
        p.on_hook(hook("UserPromptSubmit", json!({ "prompt": "Fix it" })))
            .await;
        let evs = f.events.lock().unwrap().clone();
        let untracked = evs.iter().find(|e| e["kind"] == "turn_untracked").unwrap();
        assert_eq!(
            untracked["holder"], "chat",
            "names the session that holds the project"
        );
        let prompt = evs.iter().find(|e| e["kind"] == "pane_prompt").unwrap();
        assert_eq!(
            prompt["text"], "Fix it",
            "an untracked prompt still shows its text"
        );
        assert!(!p.holds_claim());
        p.on_hook(hook("Stop", json!({}))).await;
        assert!(!kinds(&f).contains(&"turn_ended".to_string()));
        chat.release();
        // The next prompt is tracked again.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        assert!(p.holds_claim());
        p.close().await;
        assert!(!p.holds_claim(), "close releases the claim");
        assert!(!p.events_dir.exists(), "close removes the events folder");
    }

    #[tokio::test]
    async fn missed_stop_is_closed_by_the_next_prompt() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        p.on_pty_open().await;
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        // Interrupt: no Stop. The next prompt ends the old turn and starts a new one.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        let ended: Vec<Value> = f
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "turn_ended")
            .cloned()
            .collect();
        assert_eq!(ended.len(), 1);
        assert_eq!(ended[0]["reason"], "prompt");
        assert!(p.holds_claim());
        // StopFailure also ends a turn.
        p.on_hook(hook("StopFailure", json!({ "error": "rate_limit" })))
            .await;
        assert!(!p.holds_claim());
        let last = f
            .events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|e| e["kind"] == "turn_ended")
            .cloned()
            .unwrap();
        assert_eq!(last["failure"], "rate_limit");
        // Pane exit releases too.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        p.on_pty_exit("pty1").await;
        assert!(!p.holds_claim());
    }

    #[tokio::test]
    async fn without_hooks_no_turn_is_recorded() {
        let f = fixture().await;
        let p = pane(&f, None);
        p.on_pty_open().await;
        assert!(!p.holds_claim(), "no turn starts when the pane opens");
        std::fs::write(f.root.path().join("a.txt"), "two\n").unwrap();
        p.on_pty_exit("pty1").await;
        assert!(!kinds(&f).contains(&"turn_started".to_string()));
        assert!(!kinds(&f).contains(&"turn_ended".to_string()));
        // Events from another agent are ignored.
        let p = pane(&f, None);
        let mut other = hook("UserPromptSubmit", json!({}));
        other["agent"] = json!("codex");
        p.on_hook(other).await;
        assert!(!p.holds_claim());
        assert!(!kinds(&f).contains(&"hooks_live".to_string()));
        // Hooks the user added (no plugin) still report turns.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        assert!(p.holds_claim());
        assert!(kinds(&f).contains(&"hooks_live".to_string()));
    }

    #[tokio::test]
    async fn mode_key_only_while_idle() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        p.on_pty_open().await;
        assert_eq!(p.mode_key_ready(), Ok("pty1".to_string()));
        p.on_hook(hook("UserPromptSubmit", json!({ "mode": "plan" })))
            .await;
        // A prompt runs: Shift+Tab could answer a plan or permission question.
        assert_eq!(p.mode_key_ready(), Err("busy"));
        p.on_hook(hook("Stop", json!({ "mode": "plan" }))).await;
        assert_eq!(p.mode_key_ready(), Ok("pty1".to_string()));
        p.on_hook(hook("UserPromptSubmit", json!({ "mode": "auto" })))
            .await;
        let modes: Vec<Value> = f
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "pane_mode")
            .map(|e| e["mode"].clone())
            .collect();
        assert_eq!(
            modes,
            vec![json!("plan"), json!("auto")],
            "sent on change only"
        );
        p.on_pty_exit("pty1").await;
        assert_eq!(p.mode_key_ready(), Err("closed"));
        // Other agents: no mode key.
        let events = f.events.clone();
        let sink: Sink = Arc::new(move |v| events.lock().unwrap().push(v));
        let tui = TuiCommand {
            program: PathBuf::from("codex"),
            args: vec![],
        };
        let c = Pane::with_sink(sink, f.project.clone(), "codex", tui, None);
        c.set_pty(Some("pty2".into()));
        assert_eq!(c.mode_key_ready(), Err("noModeKey"));
    }

    #[tokio::test]
    async fn todo_list_from_todowrite_and_tasks() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        p.on_pty_open().await;
        let last_todos = || {
            f.events
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|e| e["kind"] == "pane_todos")
                .map(|e| e["todos"].clone())
                .unwrap()
        };
        p.on_hook(hook(
            "PostToolUse",
            json!({ "tool": "TodoWrite", "todo": { "kind": "list", "items": [
                { "content": "Add tests", "status": "completed" },
                { "content": "Fix bug", "status": "in_progress" }
            ] } }),
        ))
        .await;
        assert_eq!(last_todos()[1]["status"], "in_progress");
        // Task tools: create, update, delete. They replace a TodoWrite list.
        for todo in [
            json!({ "kind": "create", "id": "1", "subject": "Read code" }),
            json!({ "kind": "create", "id": "2", "subject": "Write code" }),
            json!({ "kind": "update", "id": "1", "status": "completed" }),
            json!({ "kind": "update", "id": "2", "status": "deleted" }),
            json!({ "kind": "update", "id": "9", "status": "completed" }),
        ] {
            p.on_hook(hook(
                "PostToolUse",
                json!({ "tool": "TaskUpdate", "todo": todo }),
            ))
            .await;
        }
        assert_eq!(
            last_todos(),
            json!([{ "content": "Read code", "status": "completed" }])
        );
    }

    #[test]
    fn take_events_reads_in_order_and_removes() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("002.json"), r#"{"n":2}"#).unwrap();
        std::fs::write(d.path().join("001.json"), r#"{"n":1}"#).unwrap();
        std::fs::write(d.path().join("003.json"), "broken").unwrap();
        std::fs::write(d.path().join("004.tmp"), r#"{"n":4}"#).unwrap();
        let evs = take_events(d.path());
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0]["n"], 1);
        assert_eq!(evs[1]["n"], 2);
        let left: Vec<_> = std::fs::read_dir(d.path()).unwrap().collect();
        assert_eq!(left.len(), 1, "only the unfinished .tmp file stays");
    }
}
