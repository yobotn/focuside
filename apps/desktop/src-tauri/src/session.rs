//! One agent session: the Claude Code process over ACP, permission requests, plan approval,
//! and turns with snapshots before and after.
//!
//! Events go to the UI as `session` events: `{ key, kind, ... }`.

use crate::project::Project;
use crate::turns::{now, TurnRecord};
use anyhow::{anyhow, bail, Result};
use focuside_core::acp::{
    self, AgentCommand, AgentProcess, Capabilities, Connection, Incoming, PermissionOption,
};
use focuside_core::proc::ProcessTree;
use focuside_core::snapshot::{rel_path, Snapshot};
use focuside_core::{agent, plan};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// How long Stop and Pause wait for the agent before killing its process tree.
const KILL_AFTER: Duration = Duration::from_secs(3);

struct Pending {
    id: Value,
    options: Vec<PermissionOption>,
    plan: bool,
}

struct TurnCtx {
    id: String,
    /// Snapshot before files could change. None while only planning.
    start: Option<Snapshot>,
    auto_accept_edits: bool,
    edits: BTreeSet<String>,
    commands: Vec<(String, String)>,
    your_edits: BTreeSet<String>,
    /// True when this prompt continues a paused turn (same record, same start).
    continued: bool,
}

#[derive(Default)]
struct State {
    acp_id: Option<String>,
    caps: Capabilities,
    initial_mode: Option<String>,
    current_mode: Option<String>,
    alive: bool,
    pending: HashMap<String, Pending>,
    tool_kinds: HashMap<String, String>,
    turn: Option<TurnCtx>,
    want_accept_edits: bool,
    undo_note: bool,
    queued: Option<String>,
}

pub struct Session {
    pub key: String,
    pub project: Arc<Project>,
    app: AppHandle,
    cmd: AgentCommand,
    conn: Mutex<Option<Connection>>,
    tree: tokio::sync::Mutex<Option<ProcessTree>>,
    start_lock: tokio::sync::Mutex<()>,
    st: Mutex<State>,
    generation: AtomicU64,
}

fn new_id(prefix: &str) -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{prefix}{ms:x}{}", N.fetch_add(1, Ordering::Relaxed))
}

impl Session {
    pub fn new(app: AppHandle, project: Arc<Project>, cmd: AgentCommand) -> Arc<Self> {
        Arc::new(Self {
            key: new_id("s"),
            project,
            app,
            cmd,
            conn: Mutex::new(None),
            tree: tokio::sync::Mutex::new(None),
            start_lock: tokio::sync::Mutex::new(()),
            st: Mutex::new(State::default()),
            generation: AtomicU64::new(0),
        })
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app
    }

    fn emit(&self, kind: &str, mut data: Value) {
        if !data.is_object() {
            data = json!({});
        }
        data["key"] = json!(self.key);
        data["kind"] = json!(kind);
        data["projectId"] = json!(self.project.id);
        if cfg!(debug_assertions) && kind != "update" {
            eprintln!("[event] {} {}", self.key, kind);
        }
        let _ = self.app.emit("session", data);
    }

    fn conn(&self) -> Result<Connection> {
        self.conn
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("The agent is not running."))
    }

    pub fn acp_id(&self) -> Option<String> {
        self.st.lock().unwrap().acp_id.clone()
    }

    pub fn is_alive(&self) -> bool {
        self.st.lock().unwrap().alive
    }

    /// Starts the agent process and opens (or resumes) the ACP session. With `replay`, a
    /// resumed session sends its history again (session/load) so the UI can show it.
    pub async fn start(self: &Arc<Self>, resume: Option<String>, replay: bool) -> Result<()> {
        let _g = self.start_lock.lock().await;
        if self.is_alive() {
            return Ok(());
        }
        self.emit("starting", json!({}));
        // Claude only with an API key or a cloud provider (Anthropic's rule for Agent SDK apps).
        if let Some(claude) = &self.cmd.claude {
            match agent::claude_auth(claude, &self.cmd.env).await {
                Ok(auth) => match auth.access() {
                    agent::ClaudeAccess::Allowed => {}
                    access => {
                        self.emit("claude_access", json!({ "access": access }));
                        bail!("claude_access");
                    }
                },
                Err(e) => tracing::warn!("claude auth status: {e:#}"),
            }
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let mut p = AgentProcess::spawn(&self.cmd, &self.project.root, None)?;
        let init = match p.conn.request("initialize", acp::initialize_params()).await {
            Ok(v) => v,
            Err(e) => {
                let tail = p.stderr_tail();
                p.tree.kill().await;
                bail!("{e}. {tail}");
            }
        };
        let caps = Capabilities::from_initialize(&init);

        // Sign-in check when the agent reports it.
        if !init["agentCapabilities"]["auth"]["status"].is_null() {
            if let Ok(s) = p.conn.request("auth/status", json!({})).await {
                if s["state"].as_str() == Some("unauthenticated") {
                    p.tree.kill().await;
                    self.emit("signin", json!({}));
                    bail!("signin");
                }
            }
        }

        let root = self.project.root.clone();
        let mut resumed = false;
        let session = match &resume {
            Some(id) => {
                let method = if caps.resume_session && !(replay && caps.load_session) {
                    "session/resume"
                } else {
                    "session/load"
                };
                match p
                    .conn
                    .request(
                        method,
                        agent::load_session_params(&root, id, caps.system_prompt),
                    )
                    .await
                {
                    Ok(mut v) => {
                        resumed = true;
                        if v["sessionId"].is_null() {
                            v["sessionId"] = json!(id);
                        }
                        Ok(v)
                    }
                    Err(e) => {
                        tracing::warn!("resume failed, starting a new session: {e}");
                        p.conn
                            .request(
                                "session/new",
                                agent::new_session_params(&root, caps.system_prompt),
                            )
                            .await
                    }
                }
            }
            None => {
                p.conn
                    .request(
                        "session/new",
                        agent::new_session_params(&root, caps.system_prompt),
                    )
                    .await
            }
        };
        let session = match session {
            Ok(s) => s,
            Err(e) => {
                let tail = p.stderr_tail();
                p.tree.kill().await;
                if e.is_auth_required() {
                    self.emit("signin", json!({}));
                    bail!("signin");
                }
                bail!("{e}. {tail}");
            }
        };
        let acp_id = session["sessionId"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mode = session["modes"]["currentModeId"]
            .as_str()
            .map(String::from)
            .or_else(|| config_value(&session["configOptions"], "mode"));
        {
            let mut st = self.st.lock().unwrap();
            st.acp_id = Some(acp_id.clone());
            st.caps = caps.clone();
            if st.initial_mode.is_none() || !resumed {
                st.initial_mode = mode
                    .clone()
                    .filter(|m| m != "plan" && m != "acceptEdits")
                    .or(Some("default".into()));
            }
            st.current_mode = mode.clone();
            st.alive = true;
        }
        *self.conn.lock().unwrap() = Some(p.conn.clone());
        let incoming = std::mem::replace(&mut p.incoming, tokio::sync::mpsc::unbounded_channel().1);
        let stderr = p.stderr.clone();
        *self.tree.lock().await = Some(p.tree);

        self.emit(
            "ready",
            json!({
                "acpId": acp_id,
                "resumed": resumed,
                "caps": caps,
                "modes": session["modes"],
                "configOptions": session["configOptions"],
                "models": session["models"],
            }),
        );

        let me = self.clone();
        tauri::async_runtime::spawn(
            async move { me.read_loop(incoming, generation, stderr).await },
        );
        Ok(())
    }

    async fn read_loop(
        self: Arc<Self>,
        mut incoming: tokio::sync::mpsc::UnboundedReceiver<Incoming>,
        generation: u64,
        stderr: Arc<Mutex<std::collections::VecDeque<String>>>,
    ) {
        while let Some(msg) = incoming.recv().await {
            if self.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            match msg {
                Incoming::Notification { method, params } if method == "session/update" => {
                    self.on_update(&params["update"]);
                    self.emit("update", json!({ "update": params["update"] }));
                }
                Incoming::Notification { method, params } => {
                    if method.ends_with("auth/status_update")
                        && params["state"].as_str() == Some("unauthenticated")
                    {
                        self.emit("signin", json!({}));
                    }
                }
                Incoming::Request { id, method, params }
                    if method == "session/request_permission" =>
                {
                    self.on_permission(id, params);
                }
                Incoming::Request { id, method, .. } => {
                    if let Ok(c) = self.conn() {
                        c.respond(id, Err(acp::RpcError::method_not_found(&method)));
                    }
                }
                Incoming::Closed => {
                    let tail = stderr
                        .lock()
                        .unwrap()
                        .iter()
                        .rev()
                        .take(6)
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    {
                        let mut st = self.st.lock().unwrap();
                        st.alive = false;
                        st.pending.clear();
                    }
                    *self.conn.lock().unwrap() = None;
                    self.emit("stopped", json!({ "detail": tail }));
                    return;
                }
            }
        }
    }

    fn on_update(self: &Arc<Self>, u: &Value) {
        let kind = u["sessionUpdate"].as_str().unwrap_or("");
        let mut switch_mode = false;
        {
            let mut st = self.st.lock().unwrap();
            match kind {
                "tool_call" | "tool_call_update" => {
                    let id = u["toolCallId"].as_str().unwrap_or("").to_string();
                    if let Some(k) = u["kind"].as_str() {
                        st.tool_kinds.insert(id.clone(), k.to_string());
                    }
                    let tk = st.tool_kinds.get(&id).cloned().unwrap_or_default();
                    let root = self.project.root.clone();
                    if let Some(turn) = st.turn.as_mut() {
                        if matches!(tk.as_str(), "edit" | "delete" | "move") {
                            for loc in u["locations"].as_array().into_iter().flatten() {
                                if let Some(p) = loc["path"].as_str() {
                                    if let Some(rel) = inside(&root, p) {
                                        turn.edits.insert(rel);
                                    }
                                }
                            }
                        }
                        // Commands count once files can change (not read-only planning).
                        if tk == "execute" && turn.start.is_some() {
                            if let Some(t) = u["title"].as_str().filter(|t| !t.is_empty()) {
                                if t != "Terminal" {
                                    let t = strip_cd(t, &root);
                                    let t = t.as_str();
                                    match turn.commands.iter_mut().find(|(i, _)| *i == id) {
                                        Some(c) => c.1 = t.to_string(),
                                        None => turn.commands.push((id.clone(), t.to_string())),
                                    }
                                }
                            }
                        }
                    }
                }
                "current_mode_update" => {
                    let m = u["currentModeId"].as_str().map(String::from);
                    if st.want_accept_edits && m.as_deref() != Some("acceptEdits") {
                        st.want_accept_edits = false;
                        switch_mode = true;
                    }
                    st.current_mode = m;
                }
                "config_option_update" => {
                    if let Some(m) = config_value(&u["configOptions"], "mode") {
                        st.current_mode = Some(m);
                    }
                }
                _ => {}
            }
        }
        if switch_mode {
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                let _ = me.set_mode("acceptEdits").await;
            });
        }
    }

    fn on_permission(&self, id: Value, params: Value) {
        let options = acp::permission_options(&params);
        let is_plan = acp::is_plan_request(&params);
        let tc_id = params["toolCall"]["toolCallId"].as_str().unwrap_or("");
        let kind = {
            let st = self.st.lock().unwrap();
            params["toolCall"]["kind"]
                .as_str()
                .map(String::from)
                .or_else(|| st.tool_kinds.get(tc_id).cloned())
                .unwrap_or_default()
        };
        // After Run plan, edits go through: that is what the user approved.
        let auto = {
            let st = self.st.lock().unwrap();
            st.turn
                .as_ref()
                .map(|t| t.auto_accept_edits)
                .unwrap_or(false)
        };
        if auto && !is_plan && matches!(kind.as_str(), "edit" | "delete" | "move") {
            if let Some(o) = options.iter().find(|o| o.kind == "allow_once") {
                if let Ok(c) = self.conn() {
                    c.respond(id, Ok(acp::permission_selected(&o.option_id)));
                    return;
                }
            }
        }
        let request_id = id.to_string();
        let mut data = json!({
            "requestId": request_id,
            "toolCall": params["toolCall"],
            "options": options,
            "isPlan": is_plan,
        });
        if is_plan {
            let text = acp::plan_text(&params).unwrap_or_default();
            let mut parsed = plan::parse(&text);
            plan::check_paths(&mut parsed, &self.project.root);
            data["planText"] = json!(text);
            data["plan"] = json!(parsed);
        }
        self.st.lock().unwrap().pending.insert(
            request_id,
            Pending {
                id,
                options,
                plan: is_plan,
            },
        );
        self.emit("permission", data);
    }

    fn take_pending(&self, request_id: &str) -> Result<Pending> {
        self.st
            .lock()
            .unwrap()
            .pending
            .remove(request_id)
            .ok_or_else(|| anyhow!("This request is no longer open."))
    }

    /// Answers a permission request with one of the agent's options.
    pub fn answer(&self, request_id: &str, option_id: &str) -> Result<()> {
        let p = self.take_pending(request_id)?;
        self.conn()?
            .respond(p.id, Ok(acp::permission_selected(option_id)));
        self.emit("permission_done", json!({ "requestId": request_id }));
        Ok(())
    }

    /// Run plan: claim the project, snapshot, then approve with edits accepted.
    pub async fn run_plan(self: &Arc<Self>, request_id: &str) -> Result<String> {
        let (options, is_plan) = {
            let st = self.st.lock().unwrap();
            let p = st
                .pending
                .get(request_id)
                .ok_or_else(|| anyhow!("This plan is no longer open."))?;
            (p.options.clone(), p.plan)
        };
        if !is_plan {
            bail!("Not a plan request.");
        }
        if !self.project.claim(&self.key) {
            bail!("otherRunning");
        }
        let turn_id = self
            .st
            .lock()
            .unwrap()
            .turn
            .as_ref()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| new_id("t"));
        let snap = {
            let repo = &self.project.shadow;
            let _g = repo.lock.lock().await;
            repo.snapshot(
                &format!("refs/fi/{}/{}/start", self.key, turn_id),
                "turn start",
            )
            .await
        };
        let snap = match snap {
            Ok(s) => s,
            Err(e) => {
                self.project.release(&self.key);
                bail!("snapshot: {e:#}");
            }
        };
        *self.project.watch_log.lock().unwrap() = Some(BTreeSet::new());
        let (option, switch) = acp::run_plan_option(&options)
            .ok_or_else(|| anyhow!("The agent offered no way to run the plan."))?;
        {
            let mut st = self.st.lock().unwrap();
            if let Some(t) = st.turn.as_mut() {
                t.start = Some(snap);
                t.auto_accept_edits = true;
            }
            st.want_accept_edits = switch;
        }
        let p = self.take_pending(request_id)?;
        self.conn()?
            .respond(p.id, Ok(acp::permission_selected(&option)));
        self.emit(
            "run_started",
            json!({ "turnId": turn_id, "requestId": request_id }),
        );
        Ok(turn_id)
    }

    /// Discard: "keep planning" and stop the turn. The plan card shows one line.
    pub fn discard_plan(self: &Arc<Self>, request_id: &str) -> Result<()> {
        let p = self.take_pending(request_id)?;
        let option = acp::reject_option(&p.options).unwrap_or_default();
        let conn = self.conn()?;
        conn.respond(p.id, Ok(acp::permission_selected(&option)));
        self.emit("permission_done", json!({ "requestId": request_id }));
        self.cancel();
        Ok(())
    }

    /// Typing while a plan waits: discard it and plan again with the new text.
    pub fn revise_plan(self: &Arc<Self>, request_id: &str, text: &str) -> Result<()> {
        self.st.lock().unwrap().queued = Some(text.to_string());
        self.discard_plan(request_id)
    }

    async fn set_mode(&self, mode: &str) -> Result<()> {
        let (acp_id, current) = {
            let st = self.st.lock().unwrap();
            (
                st.acp_id.clone().unwrap_or_default(),
                st.current_mode.clone(),
            )
        };
        if current.as_deref() == Some(mode) {
            return Ok(());
        }
        self.conn()?
            .request(
                "session/set_mode",
                json!({ "sessionId": acp_id, "modeId": mode }),
            )
            .await
            .map_err(|e| anyhow!("{e}"))?;
        self.st.lock().unwrap().current_mode = Some(mode.to_string());
        Ok(())
    }

    pub async fn set_config(&self, config_id: &str, value: Value) -> Result<Value> {
        let acp_id = self.acp_id().unwrap_or_default();
        self.conn()?
            .request(
                "session/set_config_option",
                json!({ "sessionId": acp_id, "configId": config_id, "value": value }),
            )
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    /// Sends a prompt. With `continue_turn`, the turn keeps its start snapshot (Continue after
    /// Pause). Returns the turn id.
    pub fn prompt(
        self: &Arc<Self>,
        blocks: Vec<Value>,
        plan_first: bool,
        continue_turn: Option<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
        // Boxed with a named type: prompt and finish_turn call each other.
        let me = self.clone();
        Box::pin(async move { me.prompt_inner(blocks, plan_first, continue_turn).await })
    }

    async fn prompt_inner(
        self: Arc<Self>,
        blocks: Vec<Value>,
        plan_first: bool,
        continue_turn: Option<String>,
    ) -> Result<String> {
        if self.st.lock().unwrap().turn.is_some() {
            bail!("busy");
        }
        if !self.is_alive() {
            let resume = self.acp_id();
            self.start(resume, false).await?;
        }
        // The plan instruction only matters in plan mode; other prompts keep the user's mode.
        let mode = if continue_turn.is_some() {
            "acceptEdits".to_string()
        } else if plan_first {
            "plan".to_string()
        } else {
            self.st
                .lock()
                .unwrap()
                .initial_mode
                .clone()
                .unwrap_or_else(|| "default".into())
        };
        if let Err(e) = self.set_mode(&mode).await {
            tracing::warn!("set_mode {mode}: {e}");
        }

        let continued = continue_turn.is_some();
        let turn_id = continue_turn.clone().unwrap_or_else(|| new_id("t"));
        let mut start = None;
        if !plan_first || continued {
            if !self.project.claim(&self.key) {
                bail!("otherRunning");
            }
            let existing = continue_turn.as_ref().and_then(|id| {
                self.project
                    .turns
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|t| &t.id == id)
                    .map(|t| t.start.clone())
            });
            if existing.is_none() {
                *self.project.watch_log.lock().unwrap() = Some(BTreeSet::new());
            }
            start = Some(match existing {
                Some(commit) => Snapshot {
                    commit,
                    not_covered: vec![],
                },
                None => {
                    let repo = &self.project.shadow;
                    let _g = repo.lock.lock().await;
                    match repo
                        .snapshot(
                            &format!("refs/fi/{}/{}/start", self.key, turn_id),
                            "turn start",
                        )
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            self.project.release(&self.key);
                            bail!("snapshot: {e:#}");
                        }
                    }
                }
            });
        }

        let mut blocks = blocks;
        {
            let mut st = self.st.lock().unwrap();
            if st.undo_note {
                st.undo_note = false;
                blocks.insert(
                    0,
                    json!({ "type": "text", "text": "The user reverted all file changes from your last turn.\n\n" }),
                );
            }
            st.turn = Some(TurnCtx {
                id: turn_id.clone(),
                auto_accept_edits: continued,
                start,
                edits: BTreeSet::new(),
                commands: vec![],
                your_edits: BTreeSet::new(),
                continued,
            });
        }
        let acp_id = self.acp_id().unwrap_or_default();
        let conn = self.conn()?;
        self.emit(
            "turn_started",
            json!({ "turnId": turn_id, "planFirst": plan_first, "continued": continued }),
        );
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let r = conn
                .request(
                    "session/prompt",
                    json!({ "sessionId": acp_id, "prompt": blocks }),
                )
                .await;
            me.finish_turn(r).await;
        });
        Ok(turn_id)
    }

    async fn finish_turn(self: &Arc<Self>, result: Result<Value, acp::RpcError>) {
        let Some(turn) = self.st.lock().unwrap().turn.take() else {
            return;
        };
        {
            let mut st = self.st.lock().unwrap();
            st.want_accept_edits = false;
            // Requests left open belong to the finished turn.
            st.pending.clear();
        }
        let (stop_reason, error) = match &result {
            Ok(v) => (
                v["stopReason"].as_str().unwrap_or("end_turn").to_string(),
                None,
            ),
            Err(e) => ("error".to_string(), Some(e.message.clone())),
        };
        let mut data = json!({
            "turnId": turn.id,
            "stopReason": stop_reason,
            "error": error,
            "hasSnapshot": turn.start.is_some(),
        });

        if let Some(start) = &turn.start {
            let repo = self.project.shadow.clone();
            let res = async {
                let _g = repo.lock.lock().await;
                let end = repo
                    .snapshot(&format!("refs/fi/{}/{}/end", self.key, turn.id), "turn end")
                    .await?;
                let changes = repo.diff(&start.commit, &end.commit).await?;
                // Files the watcher saw change that snapshots do not cover (ignored files).
                let seen: Vec<String> = self
                    .project
                    .watch_log
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|p| !changes.files.iter().any(|f| &f.path == p))
                    .collect();
                let mut ignored = repo.ignored(&seen).await.unwrap_or_default();
                ignored.truncate(20);
                anyhow::Ok((end, changes, ignored))
            }
            .await;
            match res {
                Ok((end, changes, ignored)) => {
                    let mut not_covered = start.not_covered.clone();
                    for n in end.not_covered {
                        if !not_covered.contains(&n) {
                            not_covered.push(n);
                        }
                    }
                    let commands: Vec<String> =
                        turn.commands.iter().map(|(_, c)| c.clone()).collect();
                    // Writes the session's own tools do not explain, in a turn without shell
                    // commands, came from somewhere else.
                    let outside = commands.is_empty()
                        && changes.files.iter().any(|f| {
                            !turn.edits.contains(&f.path) && !turn.your_edits.contains(&f.path)
                        });
                    {
                        let mut turns = self.project.turns.lock().unwrap();
                        let existing = turns.iter_mut().find(|t| t.id == turn.id);
                        match existing {
                            Some(rec) if turn.continued => {
                                rec.end = Some(end.commit.clone());
                                rec.changes = changes.clone();
                                rec.commands.extend(commands.clone());
                                rec.edits.extend(turn.edits.iter().cloned());
                                rec.your_edits.extend(turn.your_edits.iter().cloned());
                                rec.not_covered = not_covered.clone();
                                rec.undone = None;
                                for p in &ignored {
                                    if !rec.ignored_changed.contains(p) {
                                        rec.ignored_changed.push(p.clone());
                                    }
                                }
                            }
                            _ => turns.push(TurnRecord {
                                id: turn.id.clone(),
                                session_key: self.key.clone(),
                                start: start.commit.clone(),
                                end: Some(end.commit.clone()),
                                changes: changes.clone(),
                                not_covered: not_covered.clone(),
                                commands: commands.clone(),
                                edits: turn.edits.iter().cloned().collect(),
                                your_edits: turn.your_edits.iter().cloned().collect(),
                                ignored_changed: ignored.clone(),
                                undone: None,
                                created: now(),
                            }),
                        }
                    }
                    self.project.save_turns();
                    let all_commands: Vec<String> = self
                        .project
                        .turns
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|t| t.id == turn.id)
                        .map(|t| t.commands.clone())
                        .unwrap_or_default();
                    data["changes"] = json!(changes);
                    data["notCovered"] = json!(not_covered);
                    data["commands"] = json!(all_commands);
                    data["outside"] = json!(outside);
                    data["yourEdits"] = json!(turn.your_edits);
                    data["ignoredChanged"] = json!(ignored);
                }
                Err(e) => {
                    data["snapshotError"] = json!(format!("{e:#}"));
                }
            }
            self.project.release(&self.key);
            let repo = self.project.shadow.clone();
            tauri::async_runtime::spawn(async move {
                let _g = repo.lock.lock().await;
                if let Ok(dropped) = repo.prune(now()).await {
                    if !dropped.is_empty() {
                        let _ = repo.gc().await;
                    }
                }
            });
        }
        self.emit("turn_ended", data);

        let queued = self.st.lock().unwrap().queued.take();
        if let Some(text) = queued {
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                let blocks = vec![json!({ "type": "text", "text": text })];
                me.emit("revised", json!({ "text": blocks[0]["text"] }));
                if let Err(e) = me.prompt(blocks, true, None).await {
                    me.emit("error", json!({ "message": format!("{e:#}") }));
                }
            });
        }
    }

    /// Stop or Pause: cancel the prompt; kill the process tree if it has not stopped in 3 s.
    pub fn cancel(self: &Arc<Self>) {
        let (acp_id, pending, turn_id) = {
            let mut st = self.st.lock().unwrap();
            let pending: Vec<Pending> = st.pending.drain().map(|(_, p)| p).collect();
            (
                st.acp_id.clone().unwrap_or_default(),
                pending,
                st.turn.as_ref().map(|t| t.id.clone()),
            )
        };
        if let Ok(c) = self.conn() {
            for p in pending {
                c.respond(p.id, Ok(acp::permission_cancelled()));
            }
            c.notify("session/cancel", json!({ "sessionId": acp_id }));
        }
        let Some(turn_id) = turn_id else { return };
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(KILL_AFTER).await;
            let still = me
                .st
                .lock()
                .unwrap()
                .turn
                .as_ref()
                .map(|t| t.id == turn_id)
                .unwrap_or(false);
            if still {
                me.kill().await;
            }
        });
    }

    /// Kills the agent's process tree. The next prompt starts it again and resumes.
    pub async fn kill(&self) {
        if let Some(mut tree) = self.tree.lock().await.take() {
            tree.kill().await;
        }
    }

    pub async fn close(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.kill().await;
        self.st.lock().unwrap().alive = false;
        self.project.release(&self.key);
    }

    pub fn note_undo(&self, on: bool) {
        self.st.lock().unwrap().undo_note = on;
    }

    /// Records a save from the FocusIDE viewer during a running turn ("your edit").
    pub fn note_user_save(&self, rel: &str) {
        if let Some(t) = self.st.lock().unwrap().turn.as_mut() {
            t.your_edits.insert(rel.to_string());
        }
    }

    pub async fn list_sessions(&self) -> Result<Value> {
        let caps = self.st.lock().unwrap().caps.clone();
        if !caps.list_sessions {
            return Ok(json!([]));
        }
        let r = self
            .conn()?
            .request("session/list", json!({ "cwd": self.project.root }))
            .await
            .map_err(|e| anyhow!("{e}"))?;
        Ok(r["sessions"].clone())
    }
}

/// Drops a leading `cd "<project root>" && ` from a shell command: it says nothing new.
fn strip_cd(cmd: &str, root: &Path) -> String {
    let Some(rest) = cmd.strip_prefix("cd ") else {
        return cmd.to_string();
    };
    let Some((target, tail)) = rest.split_once("&&") else {
        return cmd.to_string();
    };
    let target = target.trim().trim_matches(|c| c == '"' || c == '\'');
    if inside(root, &format!("{target}/x")).as_deref() == Some("x") {
        tail.trim().to_string()
    } else {
        cmd.to_string()
    }
}

/// Current value of a config option (mode, model) from `configOptions`.
fn config_value(options: &Value, id: &str) -> Option<String> {
    options.as_array()?.iter().find_map(|o| {
        if o["id"] == id || o["category"] == id {
            o["currentValue"].as_str().map(String::from)
        } else {
            None
        }
    })
}

/// Relative path when `path` is inside `root`.
fn inside(root: &Path, path: &str) -> Option<String> {
    let p = Path::new(path);
    let lower = |s: &str| {
        if cfg!(any(windows, target_os = "macos")) {
            s.to_lowercase()
        } else {
            s.to_string()
        }
    };
    let rs = lower(&root.to_string_lossy().replace('\\', "/"));
    let ps = lower(&p.to_string_lossy().replace('\\', "/"));
    if ps.starts_with(&format!("{}/", rs.trim_end_matches('/'))) {
        let rel = rel_path(root, p);
        if rel.contains(':') || rel.starts_with('/') {
            // strip_prefix failed on case: cut by length instead.
            let full = p.to_string_lossy().replace('\\', "/");
            Some(full[rs.trim_end_matches('/').len() + 1..].to_string())
        } else {
            Some(rel)
        }
    } else {
        None
    }
}
