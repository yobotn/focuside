//! A pane session: the agent's own unmodified TUI (for example `claude`) in a PTY, shown in
//! the center view. The user types every prompt there. FocusIDE only records turns around it:
//! snapshots, changes and undo.
//!
//! Turn boundaries, Phase 2 (no hooks): a turn starts when the pane opens and right after each
//! Checkpoint, and ends on Checkpoint, pane exit and pane close. A turn holds the project
//! claim like a chat turn. When another session holds the claim, the pane's turn is not
//! tracked and the UI says so; Checkpoint tries again. Hook events (Phase 3) can call `begin`
//! and `end_turn` for UserPromptSubmit, Stop and StopFailure.
//!
//! Events go to the UI as `session` events, like chat sessions: `turn_started`, `turn_ended`,
//! `turn_untracked`, `pane_started`, `pane_exited`.

use crate::project::Project;
use crate::pty::Launch;
use crate::turn_recorder::{new_id, TurnEnd, TurnRecorder};
use focuside_core::agent::TuiCommand;
use focuside_core::snapshot::Snapshot;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

struct PaneTurn {
    id: String,
    start: Snapshot,
    your_edits: BTreeSet<String>,
}

#[derive(Default)]
struct PaneState {
    /// Id of the PTY that runs the TUI now. None before it opens and after it exits.
    pty: Option<String>,
    turn: Option<PaneTurn>,
    closed: bool,
}

pub struct Pane {
    pub key: String,
    pub project: Arc<Project>,
    /// What `pty_open` starts for this pane. Resolved in Rust, never sent by the UI.
    pub launch: Launch,
    app: AppHandle,
    recorder: TurnRecorder,
    st: Mutex<PaneState>,
    /// Turn start and end never run at the same time.
    turn_lock: tokio::sync::Mutex<()>,
}

/// The PTY launch for an agent TUI: the resolved program and arguments, no extra variables.
pub fn launch_for(tui: TuiCommand) -> Launch {
    Launch {
        program: tui.program.to_string_lossy().to_string(),
        args: tui.args,
        env: vec![],
    }
}

impl Pane {
    pub fn new(app: AppHandle, project: Arc<Project>, tui: TuiCommand) -> Arc<Self> {
        let key = new_id("s");
        Arc::new(Self {
            recorder: TurnRecorder::new(project.clone(), &key),
            key,
            project,
            launch: launch_for(tui),
            app,
            st: Mutex::new(PaneState::default()),
            turn_lock: tokio::sync::Mutex::new(()),
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
        let _ = self.app.emit("session", data);
    }

    /// Sets the PTY that runs the TUI. Exit events of other PTYs are ignored from then on.
    pub fn set_pty(&self, id: Option<String>) {
        self.st.lock().unwrap().pty = id;
    }

    fn running(&self) -> bool {
        let st = self.st.lock().unwrap();
        !st.closed && st.pty.is_some()
    }

    /// The TUI started: close a turn left open, then start one.
    pub async fn on_pty_open(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.end_locked("open").await;
        if self.running() {
            self.emit("pane_started", json!({}));
            self.begin_locked().await;
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

    /// Starts a turn if none is open and the TUI runs.
    pub async fn begin(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.begin_locked().await;
    }

    /// Ends the open turn, if any. `reason` goes to the UI ("undo", later hook events).
    /// Returns true when a turn was open.
    pub async fn end_turn(self: &Arc<Self>, reason: &str) -> bool {
        let _g = self.turn_lock.lock().await;
        self.end_locked(reason).await
    }

    /// Checkpoint button: ends the open turn and starts the next one. Also starts tracking
    /// again after a turn that was not tracked.
    pub async fn checkpoint(self: &Arc<Self>) {
        let _g = self.turn_lock.lock().await;
        self.end_locked("checkpoint").await;
        self.begin_locked().await;
    }

    async fn begin_locked(self: &Arc<Self>) {
        if !self.running() || self.st.lock().unwrap().turn.is_some() {
            return;
        }
        if !self.recorder.claim() {
            self.emit("turn_untracked", json!({ "reason": "otherRunning" }));
            return;
        }
        let id = new_id("t");
        match self.recorder.start(&id).await {
            Ok(start) => {
                self.st.lock().unwrap().turn = Some(PaneTurn {
                    id: id.clone(),
                    start,
                    your_edits: BTreeSet::new(),
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
        });
        let none = BTreeSet::new();
        let res = self
            .recorder
            .finish(TurnEnd {
                id: &turn.id,
                start: &turn.start,
                edits: &none,
                commands: vec![],
                your_edits: &turn.your_edits,
                continued: false,
                tools_known: false,
            })
            .await;
        match res {
            Ok(r) => r.write_to(&mut data),
            Err(e) => data["snapshotError"] = json!(format!("{e:#}")),
        }
        self.emit("turn_ended", data);
        true
    }

    /// Closes the pane session: ends the open turn and releases the claim. The caller kills
    /// the PTY.
    pub async fn close(self: &Arc<Self>) {
        {
            let mut st = self.st.lock().unwrap();
            st.closed = true;
            st.pty = None;
        }
        let _g = self.turn_lock.lock().await;
        self.end_locked("close").await;
        self.recorder.release();
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
