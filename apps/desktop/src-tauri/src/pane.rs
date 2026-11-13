//! A pane session: the agent's own unmodified TUI (for example `claude`) in a PTY, shown in
//! the center view. The user types every prompt there. FocusIDE only records turns around it:
//! snapshots, changes and undo.
//!
//! Every pane gets `FOCUSIDE_EVENTS`, a folder in app data where the `focuside-hook` helper
//! writes hook events (see hooks.rs). A watcher on that folder feeds `on_hook`.
//!
//! Turn boundaries:
//! - Without hooks (no event seen yet, and no plugin passed): a turn starts when the pane
//!   opens and right after each Checkpoint, and ends on Checkpoint, pane exit and pane close.
//!   The pane holds the project claim while its turn is open (Phase 2).
//! - With hooks (an event arrived): a turn starts on UserPromptSubmit and ends on Stop or
//!   StopFailure. The claim is held only in between. It is also released by the next
//!   UserPromptSubmit (Stop does not fire after an interrupt), Checkpoint, pane exit and pane
//!   close. A prompt that arrives while another session holds the claim records no turn.
//! - A Claude pane started with the plugin expects hooks, so it starts no turn when it opens.
//!   Until the first event arrives, Checkpoint starts a turn by hand, as without hooks.
//!
//! Events go to the UI as `session` events, like chat sessions: `turn_started`, `turn_ended`,
//! `turn_untracked`, `pane_started`, `pane_exited`, `pane_idle`, `pane_plan`, `hooks_live`.

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

/// Where a turn's boundaries come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Pane open or Checkpoint (no hooks).
    Manual,
    /// UserPromptSubmit.
    Hook,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Source::Manual => "manual",
            Source::Hook => "hook",
        }
    }
}

struct PaneTurn {
    id: String,
    source: Source,
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
    sink: Sink,
    recorder: TurnRecorder,
    st: Mutex<PaneState>,
    /// Turn start and end never run at the same time.
    turn_lock: tokio::sync::Mutex<()>,
    /// Keeps the events folder watcher alive.
    watcher: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

/// The PTY launch for an agent TUI: the resolved program and arguments, plus
/// `FOCUSIDE_EVENTS`. With `plugin`, `--plugin-dir <plugin>` is added (Claude only, after the
/// user agreed). Nothing else is added and nothing is removed.
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

    /// True when turns come from hooks: an event arrived, or the plugin was passed.
    fn hook_mode(&self) -> bool {
        self.hooks_expected || self.st.lock().unwrap().hooks_live
    }

    /// The TUI started: close a turn left open, then start one unless hooks start turns.
    pub async fn on_pty_open(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.end_locked("open").await;
        if self.running() {
            self.emit(
                "pane_started",
                json!({ "hooksExpected": self.hooks_expected }),
            );
            if self.hook_mode() {
                self.emit("pane_idle", json!({}));
            } else {
                self.begin_locked(Source::Manual).await;
            }
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
        let first = {
            let mut st = self.st.lock().unwrap();
            if st.closed {
                return;
            }
            !std::mem::replace(&mut st.hooks_live, true)
        };
        if first {
            self.emit("hooks_live", json!({}));
        }
        match event {
            "UserPromptSubmit" => {
                // Also closes a turn whose Stop never came (interrupt) or a Checkpoint turn.
                self.end_locked("prompt").await;
                self.begin_locked(Source::Hook).await;
            }
            "Stop" | "StopFailure" => {
                let failure = ev["error"].as_str().map(String::from);
                if let Some(t) = self.st.lock().unwrap().turn.as_mut() {
                    if event == "Stop" {
                        t.last_message = ev["lastMessage"].as_str().map(String::from);
                    } else {
                        t.failure = failure.clone().or(Some(String::new()));
                    }
                }
                let reason = if event == "Stop" {
                    "stop"
                } else {
                    "stopFailure"
                };
                self.end_locked(reason).await;
                self.emit("pane_idle", json!({ "failure": failure }));
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

    /// Starts a turn by hand (after undo) if none is open and the TUI runs.
    pub async fn begin(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.begin_locked(Source::Manual).await;
    }

    /// Ends the open turn, if any. `reason` goes to the UI. Returns true when a turn was open.
    pub async fn end_turn(self: &Arc<Self>, reason: &str) -> bool {
        let _g = self.turn_lock.lock().await;
        self.end_locked(reason).await
    }

    /// Undo and redo need the project free. A turn started by a hook belongs to a running
    /// prompt: like a chat turn, it has to finish first (Err). A turn started by hand ends
    /// here; Ok(true) tells the caller to start the next one after the undo.
    pub async fn end_for_undo(self: &Arc<Self>) -> Result<bool, ()> {
        let _g = self.turn_lock.lock().await;
        let source = self.st.lock().unwrap().turn.as_ref().map(|t| t.source);
        match source {
            None => Ok(false),
            Some(Source::Hook) => Err(()),
            Some(Source::Manual) => {
                self.end_locked("undo").await;
                Ok(true)
            }
        }
    }

    /// Checkpoint button: ends the open turn. Without hook events it also starts the next one;
    /// with hooks the next prompt does. Also starts tracking again after an untracked turn.
    pub async fn checkpoint(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.end_locked("checkpoint").await;
        if self.st.lock().unwrap().hooks_live {
            self.emit("pane_idle", json!({}));
        } else {
            self.begin_locked(Source::Manual).await;
        }
    }

    async fn begin_locked(self: &Arc<Self>, source: Source) {
        if !self.running() || self.st.lock().unwrap().turn.is_some() {
            return;
        }
        if !self.recorder.claim() {
            self.emit(
                "turn_untracked",
                json!({ "reason": "otherRunning", "source": source.name() }),
            );
            return;
        }
        let id = new_id("t");
        match self.recorder.start(&id).await {
            Ok(start) => {
                self.st.lock().unwrap().turn = Some(PaneTurn {
                    id: id.clone(),
                    source,
                    start,
                    your_edits: BTreeSet::new(),
                    edits: BTreeSet::new(),
                    commands: vec![],
                    plan_steps: vec![],
                    last_message: None,
                    failure: None,
                });
                self.emit(
                    "turn_started",
                    json!({ "turnId": id, "pane": true, "source": source.name() }),
                );
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
            "source": turn.source.name(),
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
                tools_known: turn.source == Source::Hook,
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
        assert!(!p.holds_claim(), "Stop releases the claim");
        assert!(chat.claim());
        chat.release();

        let evs = f.events.lock().unwrap().clone();
        let ended = evs.iter().find(|e| e["kind"] == "turn_ended").unwrap();
        assert_eq!(ended["source"], "hook");
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
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        assert!(kinds(&f).contains(&"turn_untracked".to_string()));
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
    async fn missed_stop_is_closed_by_the_next_prompt_or_checkpoint() {
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
        // Undo has to wait for a hook turn.
        assert!(p.end_for_undo().await.is_err());
        // Checkpoint releases it and, with hooks, starts nothing.
        p.checkpoint().await;
        assert!(!p.holds_claim());
        // StopFailure also ends a turn.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
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
    async fn without_hooks_phase_2_rules_hold() {
        let f = fixture().await;
        let p = pane(&f, None);
        p.on_pty_open().await;
        assert!(
            p.holds_claim(),
            "no plugin: the turn starts when the pane opens"
        );
        assert_eq!(p.end_for_undo().await, Ok(true));
        assert!(!p.holds_claim());
        p.begin().await;
        p.checkpoint().await;
        assert!(p.holds_claim(), "Checkpoint starts the next turn");
        // Events from another agent are ignored.
        let mut other = hook("Stop", json!({}));
        other["agent"] = json!("codex");
        p.on_hook(other).await;
        assert!(p.holds_claim());
        // The first hook event switches to hook turns: the prompt ends the manual turn.
        p.on_hook(hook("UserPromptSubmit", json!({}))).await;
        p.on_hook(hook("Stop", json!({}))).await;
        assert!(!p.holds_claim());
        p.checkpoint().await;
        assert!(!p.holds_claim(), "with hooks, Checkpoint starts nothing");
        assert!(kinds(&f).contains(&"hooks_live".to_string()));
    }

    #[tokio::test]
    async fn expected_hooks_that_never_run_fall_back_to_checkpoint() {
        let f = fixture().await;
        let p = pane(&f, Some(PathBuf::from("/plugin")));
        p.on_pty_open().await;
        assert!(!p.holds_claim());
        p.checkpoint().await;
        assert!(p.holds_claim(), "Checkpoint starts a turn by hand");
        p.checkpoint().await;
        assert!(p.holds_claim());
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
