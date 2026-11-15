//! Tauri commands. Errors go to the UI as plain strings.

use crate::pane::Pane;
use crate::project::{Project, ProjectInfo};
use crate::session::Session;
use crate::turns;
use crate::AppState;
use focuside_core::agent;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn project(state: &AppState, id: &str) -> Res<Arc<Project>> {
    state
        .projects
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| "This project is not open.".to_string())
}

fn pane(state: &AppState, key: &str) -> Res<Arc<Pane>> {
    state
        .panes
        .lock()
        .unwrap()
        .get(key)
        .cloned()
        .ok_or_else(|| "This session is closed.".to_string())
}

/// "missing:<what>" for the UI, which shows the install hint or the download.
fn missing(m: agent::Missing) -> String {
    match m {
        agent::Missing::Claude => "missing:claude".to_string(),
        agent::Missing::Adapter => "missing:adapter".to_string(),
        agent::Missing::Agent => "missing:agent".to_string(),
    }
}

fn session(state: &AppState, key: &str) -> Res<Arc<Session>> {
    state
        .sessions
        .lock()
        .unwrap()
        .get(key)
        .cloned()
        .ok_or_else(|| "This session is closed.".to_string())
}

/// Dev builds: UI errors and debug lines go to stderr.
#[tauri::command]
pub fn debug_log(message: String) {
    eprintln!("[ui] {message}");
}

/// What is installed: Claude Code, git and the claude-agent-acp adapter. `adapterDownload` names
/// the release FocusIDE would download when the adapter is missing.
#[tauri::command]
pub fn env_check(state: State<'_, AppState>) -> Value {
    use focuside_core::adapter_download as dl;
    json!({
        "claude": agent::locate_claude(),
        "git": state.git,
        "adapter": agent::locate_adapter(),
        "adapterDownload": dl::current().map(|a| json!({
            "repo": dl::REPO,
            "version": dl::VERSION,
            "sizeMb": a.size.div_ceil(1_000_000),
        })),
        "os": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// The ACP agents FocusIDE knows and whether each is installed.
#[tauri::command]
pub async fn agent_list() -> Vec<agent::AgentInfo> {
    tauri::async_runtime::spawn_blocking(agent::agents)
        .await
        .unwrap_or_default()
}

/// Downloads the pinned claude-agent-acp release from GitHub into app data, after the user
/// agreed in the UI. Progress arrives as `adapter-download` events. Returns the adapter path.
#[tauri::command]
pub async fn adapter_download(app: AppHandle, state: State<'_, AppState>) -> Res<String> {
    use tauri::Emitter;
    let root = state.data_dir.clone();
    let task = tauri::async_runtime::spawn(async move {
        let mut last = u64::MAX;
        focuside_core::adapter_download::download(&root, |received, total| {
            let pct = received * 100 / total.max(1);
            if pct != last {
                last = pct;
                let _ = app.emit(
                    "adapter-download",
                    json!({ "received": received, "total": total }),
                );
            }
        })
        .await
    });
    let path = task.await.map_err(err)?.map_err(|e| format!("{e:#}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn startup_paths(state: State<'_, AppState>) -> Vec<String> {
    std::mem::take(&mut *state.startup_paths.lock().unwrap())
}

/// The UI keeps its own state (recent projects, session names, settings) as one JSON file.
/// A file that does not parse (for example after a hand edit) is kept as state.json.bad, and
/// the previous good state is tried from state.json.bak, so the next save never wipes it.
/// It holds, per project, the open session tabs (order, active tab), the saved sessions with
/// their names and agent session ids, the panel layout and which panels were shown.
#[tauri::command]
pub fn ui_state_load(state: State<'_, AppState>) -> Value {
    load_state(&state.config_dir)
}

#[tauri::command]
pub fn ui_state_save(state: State<'_, AppState>, value: Value) -> Res<()> {
    save_state(&state.config_dir, &value).map_err(err)
}

fn load_state(dir: &std::path::Path) -> Value {
    let path = dir.join("state.json");
    let parse = |p: &std::path::Path| -> Option<Value> {
        let b = std::fs::read(p).ok()?;
        let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&b);
        serde_json::from_slice(b).ok()
    };
    if let Some(v) = parse(&path) {
        return v;
    }
    if path.is_file() {
        let bad = dir.join("state.json.bad");
        let _ = std::fs::copy(&path, &bad);
        tracing::warn!(
            "state.json does not parse; kept a copy at {}",
            bad.display()
        );
    }
    parse(&dir.join("state.json.bak")).unwrap_or(Value::Null)
}

fn save_state(dir: &std::path::Path, value: &Value) -> std::io::Result<()> {
    let path = dir.join("state.json");
    let tmp = dir.join("state.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    // One previous version stays as state.json.bak.
    if path.is_file() {
        let _ = std::fs::copy(&path, dir.join("state.json.bak"));
    }
    std::fs::rename(&tmp, &path)
}

#[tauri::command]
pub async fn project_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Res<ProjectInfo> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("Folder not found: {path}"));
    }
    let root = PathBuf::from(crate::dunce(root.canonicalize().unwrap_or(root)));
    let id = focuside_core::project::project_id(&root);
    if let Some(p) = state.projects.lock().unwrap().get(&id) {
        return Ok(p.info());
    }
    let git = state
        .git
        .clone()
        .ok_or_else(|| "Git is not installed.".to_string())?;
    let p = Project::open(&root, &state.data_dir, &git)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let skip: Vec<String> = state.plan_folder.lock().unwrap().iter().cloned().collect();
    p.shadow.set_skip_dirs(skip);
    state.projects.lock().unwrap().insert(id, p.clone());
    // Watching a big tree takes a moment; never hold up opening the project.
    let watched = p.clone();
    tauri::async_runtime::spawn_blocking(move || crate::files::watch(app, &watched));
    // Snapshot when the project opens, in the background, so the first turn is fast.
    let repo = p.shadow.clone();
    tauri::async_runtime::spawn(async move {
        let _g = repo.lock.lock().await;
        if let Err(e) = repo.snapshot("refs/fi/opened", "project opened").await {
            tracing::warn!("open snapshot: {e:#}");
        }
    });
    Ok(p.info())
}

#[tauri::command]
pub async fn project_close(state: State<'_, AppState>, id: String) -> Res<()> {
    let sessions: Vec<Arc<Session>> = state
        .sessions
        .lock()
        .unwrap()
        .values()
        .filter(|s| s.project.id == id)
        .cloned()
        .collect();
    for s in sessions {
        s.close().await;
        state.sessions.lock().unwrap().remove(&s.key);
    }
    let panes: Vec<Arc<Pane>> = state
        .panes
        .lock()
        .unwrap()
        .values()
        .filter(|p| p.project.id == id)
        .cloned()
        .collect();
    for p in panes {
        close_pane(&state, &p).await;
    }
    state.projects.lock().unwrap().remove(&id);
    Ok(())
}

/// Creates a session and starts its agent in the background. Progress arrives as events.
/// `mode` "pane" creates a pane session instead: the agent's own TUI, which the UI starts with
/// `pty_open` and this session key. For a pane, `resume` is the agent's own session id (from
/// its hook events, saved with the session): the TUI starts with the agent's resume option
/// (`agent::tui_resume_args`) and falls back to a fresh TUI when that fails. Without an id, or
/// for an agent without a resume option, the pane starts a fresh TUI. For chat mode, `resume`
/// is the ACP session id (session/load). `hooks` is the "Turn tracking in Claude panes" setting (on by default): only
/// with true does a Claude pane get `--plugin-dir` with the generated focuside plugin. `brief`
/// is "Brief replies in Claude panes" (off by default): the plugin copy with the output style.
/// `plan_first` starts a Claude pane in plan mode (`--permission-mode plan`, see
/// `agent::tui_plan_args`). `plan_folder` is Settings > "Plan folder" inside the project: a
/// Claude pane gets `--settings` with only `plansDirectory` (see `agent::plans_dir_args`).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn session_start(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    resume: Option<String>,
    agent_id: Option<String>,
    custom_command: Option<String>,
    mode: Option<String>,
    hooks: Option<bool>,
    plan_first: Option<bool>,
    brief: Option<bool>,
    plan_folder: Option<String>,
) -> Res<String> {
    let p = project(&state, &project_id)?;
    if mode.as_deref() == Some("pane") {
        let agent_id = agent_id.as_deref().unwrap_or("claude");
        let tui = agent::tui_command(agent_id).map_err(missing)?;
        // Plan mode and the plan folder go after the resume option (see `Pane::launch`).
        let mut extra = agent::TuiCommand {
            program: tui.program.clone(),
            args: vec![],
        };
        if plan_first == Some(true) {
            extra.start_in_plan_mode(agent_id);
        }
        if let Some(f) = plan_folder.as_deref() {
            extra.set_plans_folder(agent_id, f);
        }
        let plugin = if agent_id == "claude" && hooks == Some(true) {
            claude_plugin(&state.data_dir, brief == Some(true))
        } else {
            None
        };
        let start = crate::pane::PaneLaunch {
            tui,
            extra_args: extra.args,
            plugin,
            resume,
        };
        let pane = Pane::new(app, p, agent_id, start);
        let key = pane.key.clone();
        state.panes.lock().unwrap().insert(key.clone(), pane);
        return Ok(key);
    }
    let cmd = agent::agent_command(
        agent_id.as_deref().unwrap_or("claude"),
        custom_command.as_deref(),
    )
    .map_err(missing)?;
    let s = Session::new(app, p, cmd);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(s.key.clone(), s.clone());
    let key = s.key.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = s.start(resume, true).await {
            let msg = format!("{e:#}");
            if msg != "signin" && msg != "claude_route" {
                use tauri::Emitter;
                let _ = s.app_handle().emit(
                    "session",
                    json!({ "key": s.key, "projectId": s.project.id, "kind": "failed", "message": msg }),
                );
            }
        }
    });
    Ok(key)
}

/// Sends a prompt. `blocks` are ACP content blocks (text, image, resource_link).
#[tauri::command]
pub async fn session_prompt(
    state: State<'_, AppState>,
    key: String,
    blocks: Vec<Value>,
    plan_first: bool,
    continue_turn: Option<String>,
) -> Res<String> {
    let s = session(&state, &key)?;
    s.prompt(blocks, plan_first, continue_turn)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn session_answer(
    state: State<'_, AppState>,
    key: String,
    request_id: String,
    option_id: String,
) -> Res<()> {
    session(&state, &key)?
        .answer(&request_id, &option_id)
        .map_err(err)
}

#[tauri::command]
pub async fn session_run_plan(
    state: State<'_, AppState>,
    key: String,
    request_id: String,
) -> Res<String> {
    session(&state, &key)?
        .run_plan(&request_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn session_discard_plan(
    state: State<'_, AppState>,
    key: String,
    request_id: String,
) -> Res<()> {
    session(&state, &key)?
        .discard_plan(&request_id)
        .map_err(err)
}

#[tauri::command]
pub fn session_revise_plan(
    state: State<'_, AppState>,
    key: String,
    request_id: String,
    text: String,
) -> Res<()> {
    session(&state, &key)?
        .revise_plan(&request_id, &text)
        .map_err(err)
}

/// Starts a stopped agent again and resumes its session ("Agent stopped. Resume").
#[tauri::command]
pub async fn session_resume(state: State<'_, AppState>, key: String) -> Res<()> {
    let s = session(&state, &key)?;
    let id = s.acp_id();
    s.start(id, false).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn session_cancel(state: State<'_, AppState>, key: String) -> Res<()> {
    session(&state, &key)?.cancel();
    Ok(())
}

#[tauri::command]
pub async fn session_close(state: State<'_, AppState>, key: String) -> Res<()> {
    let s = state.sessions.lock().unwrap().remove(&key);
    if let Some(s) = s {
        s.close().await;
    }
    let pane = state.panes.lock().unwrap().get(&key).cloned();
    if let Some(p) = pane {
        close_pane(&state, &p).await;
    }
    Ok(())
}

/// Ends the pane's open turn, then kills its TUI and the processes it started.
async fn close_pane(state: &AppState, p: &Arc<Pane>) {
    state.panes.lock().unwrap().remove(&p.key);
    p.close().await;
    crate::pty::close_session(state, &p.key);
}

/// Writes the focuside plugin folder for this OS. None (and a warning) when the helper is
/// missing or the folder cannot be written; the pane then runs without hooks.
fn claude_plugin(data_dir: &std::path::Path, brief: bool) -> Option<PathBuf> {
    let Some(helper) = crate::hooks::helper_path() else {
        tracing::warn!("focuside-hook not found next to the app; Claude pane runs without hooks");
        return None;
    };
    crate::hooks::ensure_plugin(data_dir, &helper, brief)
        .map_err(|e| tracing::warn!("focuside plugin: {e:#}"))
        .ok()
}

/// What Settings shows about hooks: whether the helper is there, and the Codex and Gemini CLI
/// snippets for this OS with the helper's path. FocusIDE never writes those files itself.
#[tauri::command]
pub fn hooks_info() -> Value {
    let helper = crate::hooks::helper_path().map(crate::dunce);
    let windows = cfg!(windows);
    json!({
        "helper": helper,
        "codex": helper.as_deref().map(|h| crate::hooks::codex_snippet(h, windows)),
        "gemini": helper.as_deref().map(|h| crate::hooks::gemini_snippet(h, windows)),
    })
}

/// The mode button of a Claude pane: sends one Shift+Tab key sequence (ESC [ Z) to the pane's
/// TUI, the same input as the user pressing Shift+Tab there. Claude Code moves to its next
/// permission mode. Only this fixed sequence is sent, never text. Refused while a prompt runs
/// ("busy"), because Shift+Tab answers an open plan or permission question.
#[tauri::command]
pub fn pane_mode_key(state: State<'_, AppState>, key: String) -> Res<()> {
    let pty_id = pane(&state, &key)?.mode_key_ready().map_err(String::from)?;
    let ptys = state.ptys.lock().unwrap();
    let pty = ptys.get(&pty_id).ok_or("closed")?;
    pty.write(crate::pane::MODE_KEY).map_err(|e| e.to_string())
}

/// Settings > "Plan folder": None or "" for Claude's default folder, else a folder relative to
/// the project root. Snapshots and the turn watch log of every project skip it from now on.
/// Returns the cleaned folder. Err when the folder is not inside the project.
#[tauri::command]
pub fn plan_folder_set(state: State<'_, AppState>, folder: Option<String>) -> Res<Option<String>> {
    let clean = match folder.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(f) => Some(agent::clean_plans_folder(f).ok_or("outside")?),
    };
    *state.plan_folder.lock().unwrap() = clean.clone();
    let skip: Vec<String> = clean.iter().cloned().collect();
    for p in state.projects.lock().unwrap().values() {
        p.shadow.set_skip_dirs(skip.clone());
    }
    Ok(clean)
}

/// Whether the project's git ignores the plan folder: `{ "ignored": true | false | null }`.
/// Null when the project is not a git repo or the folder is not a valid plan folder.
#[tauri::command]
pub async fn plan_folder_status(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> Res<Value> {
    let p = project(&state, &project_id)?;
    let (Some(git), Some(f)) = (state.git.clone(), agent::clean_plans_folder(&folder)) else {
        return Ok(json!({ "ignored": null }));
    };
    let ignored = crate::plan_folder::ignored(&git, &p.root, &f).await;
    Ok(json!({ "ignored": ignored, "line": crate::plan_folder::gitignore_line(&f) }))
}

/// "Add to .gitignore" in Settings: adds the plan folder to the project's `.gitignore`. Runs
/// only on that click; FocusIDE never edits `.gitignore` by itself.
#[tauri::command]
pub fn plan_folder_gitignore(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> Res<()> {
    let p = project(&state, &project_id)?;
    let f = agent::clean_plans_folder(&folder).ok_or("outside")?;
    crate::plan_folder::add_to_gitignore(&p.root, &f).map_err(|e| format!("{e:#}"))
}

/// Saved data of one session (plan versions, prompts and answers), or null. See session_data.rs.
#[tauri::command]
pub fn session_data_load(state: State<'_, AppState>, project_id: String, id: String) -> Res<Value> {
    let p = project(&state, &project_id)?;
    Ok(crate::session_data::load(&p.data_dir, &id))
}

#[tauri::command]
pub fn session_data_save(
    state: State<'_, AppState>,
    project_id: String,
    id: String,
    value: Value,
) -> Res<()> {
    let p = project(&state, &project_id)?;
    crate::session_data::save(&p.data_dir, &id, &value).map_err(|e| format!("{e:#}"))
}

/// How the user's Claude Code is signed in, as a route: chat (API key or cloud provider) or
/// pane (Claude plan or signed out). Null when `claude` is missing or the check failed. Only
/// the method and provider are read (`claude auth status --json`).
#[tauri::command]
pub async fn claude_route() -> Value {
    let Some(claude) = agent::locate_claude() else {
        return Value::Null;
    };
    match agent::claude_auth(&claude, &[]).await {
        Ok(auth) => {
            let access = auth.access();
            json!({ "access": access, "route": access.route() })
        }
        Err(e) => {
            tracing::warn!("claude auth status: {e:#}");
            Value::Null
        }
    }
}

#[tauri::command]
pub async fn session_set_config(
    state: State<'_, AppState>,
    key: String,
    config_id: String,
    value: Value,
) -> Res<Value> {
    session(&state, &key)?
        .set_config(&config_id, value)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn session_list(state: State<'_, AppState>, key: String) -> Res<Value> {
    session(&state, &key)?
        .list_sessions()
        .await
        .map_err(|e| format!("{e:#}"))
}

fn session_of_turn(state: &AppState, project_id: &str, turn_id: &str) -> Option<Arc<Session>> {
    let p = state.projects.lock().unwrap().get(project_id).cloned()?;
    let key = p
        .turns
        .lock()
        .unwrap()
        .iter()
        .find(|t| t.id == turn_id)
        .map(|t| t.session_key.clone())?;
    state.sessions.lock().unwrap().get(&key).cloned()
}

/// Undo and redo need the project free. A chat turn must finish first, and so must a pane
/// turn (a prompt that runs).
fn ensure_free(p: &Project) -> Res<()> {
    match p.running_session() {
        Some(_) => Err("Wait for the running turn to finish.".into()),
        None => Ok(()),
    }
}

#[tauri::command]
pub async fn turn_undo_preview(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
) -> Res<turns::UndoPreview> {
    let p = project(&state, &project_id)?;
    turns::preview(&p, &turn_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn turn_undo(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    revert_anyway: Vec<String>,
) -> Res<turns::UndoOutcome> {
    let p = project(&state, &project_id)?;
    ensure_free(&p)?;
    let out = turns::undo(&p, &turn_id, &revert_anyway)
        .await
        .map_err(|e| format!("{e:#}"))?;
    // Chat sessions tell the agent in the next prompt. Panes never get text typed in.
    if let Some(s) = session_of_turn(&state, &project_id, &turn_id) {
        s.note_undo(true);
    }
    Ok(out)
}

#[tauri::command]
pub async fn turn_redo(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
) -> Res<turns::UndoOutcome> {
    let p = project(&state, &project_id)?;
    ensure_free(&p)?;
    let out = turns::redo(&p, &turn_id)
        .await
        .map_err(|e| format!("{e:#}"))?;
    if let Some(s) = session_of_turn(&state, &project_id, &turn_id) {
        s.note_undo(false);
    }
    Ok(out)
}

#[tauri::command]
pub async fn turn_revert_file(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    path: String,
    force: Option<bool>,
) -> Res<turns::UndoOutcome> {
    let p = project(&state, &project_id)?;
    ensure_free(&p)?;
    turns::revert_file(&p, &turn_id, &path, force.unwrap_or(false))
        .await
        .map_err(|e| format!("{e:#}"))
}

/// One file of a turn for the review: both versions, the hunks and their state on disk.
#[tauri::command]
pub async fn turn_file_diff(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    path: String,
) -> Res<focuside_core::hunks::FileDiff> {
    let p = project(&state, &project_id)?;
    turns::file_diff(&p, &turn_id, &path)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// Undo change: one hunk back to how it was before the turn. Rejects with "changedSince" when
/// those lines changed after the turn.
#[tauri::command]
pub async fn turn_undo_hunk(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    path: String,
    index: usize,
) -> Res<focuside_core::hunks::FileDiff> {
    let p = project(&state, &project_id)?;
    ensure_free(&p)?;
    turns::undo_hunk(&p, &turn_id, &path, index)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn turn_file_versions(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    path: String,
) -> Res<turns::FileVersions> {
    let p = project(&state, &project_id)?;
    turns::versions(&p, &turn_id, &path)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// One side of a changed image in a turn, as raw bytes, for the diff viewer.
#[tauri::command]
pub async fn turn_file_bytes(
    state: State<'_, AppState>,
    project_id: String,
    turn_id: String,
    path: String,
    after: bool,
) -> Res<tauri::ipc::Response> {
    let p = project(&state, &project_id)?;
    let bytes = turns::image_bytes(&p, &turn_id, &path, after)
        .await
        .map_err(|e| format!("{e:#}"))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the UI saves for a restart: tabs per project, the saved sessions with the agent's
    /// session id, panel visibility and the dock. It must come back as it was saved.
    #[test]
    fn ui_state_round_trip() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load_state(d.path()), Value::Null);
        let state = json!({
            "open": [r"C:\p"],
            "active": r"C:\p",
            "tabs": { r"C:\p": { "ids": ["pane-s2", "chat-1"], "active": "chat-1" } },
            "panels": { "plan": true, "prompts": true, "terminal": false },
            "sessions": { r"C:\p": [
                { "acpId": "pane-s2", "name": "Fix the bug", "mode": "pane", "agent": "claude",
                  "agentSession": "0b9d7d0e-5d43-4c1e-9a4f-3f7e2a1c9b10" },
                { "acpId": "chat-1", "name": "Tests", "mode": "chat", "agent": "claude" }
            ] },
            "filesVisible": false,
            "layout": { "sideView": "sessions", "dock": { "left": [] } },
        });
        save_state(d.path(), &state).unwrap();
        assert_eq!(load_state(d.path()), state);
        // A second save keeps the first as state.json.bak; a broken state.json falls back to it.
        let mut next = state.clone();
        next["tabs"][r"C:\p"]["active"] = json!("pane-s2");
        save_state(d.path(), &next).unwrap();
        assert_eq!(load_state(d.path()), next);
        std::fs::write(d.path().join("state.json"), b"{ broken").unwrap();
        assert_eq!(load_state(d.path()), state);
        assert!(d.path().join("state.json.bad").is_file());
    }
}
