//! Tauri commands. Errors go to the UI as plain strings.

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

/// What is installed: Claude Code, git and the bundled ACP adapter.
#[tauri::command]
pub fn env_check(state: State<'_, AppState>) -> Value {
    json!({
        "claude": agent::locate_claude(),
        "git": state.git,
        "adapter": agent::locate_adapter(),
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

#[tauri::command]
pub fn startup_paths(state: State<'_, AppState>) -> Vec<String> {
    std::mem::take(&mut *state.startup_paths.lock().unwrap())
}

/// The UI keeps its own state (recent projects, session names, settings) as one JSON file.
/// A file that does not parse (for example after a hand edit) is kept as state.json.bad, and
/// the previous good state is tried from state.json.bak, so the next save never wipes it.
#[tauri::command]
pub fn ui_state_load(state: State<'_, AppState>) -> Value {
    let path = state.config_dir.join("state.json");
    let parse = |p: &std::path::Path| -> Option<Value> {
        let b = std::fs::read(p).ok()?;
        let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&b);
        serde_json::from_slice(b).ok()
    };
    if let Some(v) = parse(&path) {
        return v;
    }
    if path.is_file() {
        let bad = state.config_dir.join("state.json.bad");
        let _ = std::fs::copy(&path, &bad);
        tracing::warn!(
            "state.json does not parse; kept a copy at {}",
            bad.display()
        );
    }
    parse(&state.config_dir.join("state.json.bak")).unwrap_or(Value::Null)
}

#[tauri::command]
pub fn ui_state_save(state: State<'_, AppState>, value: Value) -> Res<()> {
    let path = state.config_dir.join("state.json");
    let tmp = state.config_dir.join("state.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&value).map_err(err)?).map_err(err)?;
    // One previous version stays as state.json.bak.
    if path.is_file() {
        let _ = std::fs::copy(&path, state.config_dir.join("state.json.bak"));
    }
    std::fs::rename(&tmp, &path).map_err(err)
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
    state.projects.lock().unwrap().remove(&id);
    Ok(())
}

/// Creates a session and starts its agent in the background. Progress arrives as events.
#[tauri::command]
pub async fn session_start(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    resume: Option<String>,
    agent_id: Option<String>,
    custom_command: Option<String>,
) -> Res<String> {
    let p = project(&state, &project_id)?;
    let cmd = agent::agent_command(
        agent_id.as_deref().unwrap_or("claude"),
        custom_command.as_deref(),
    )
    .map_err(|m| match m {
        agent::Missing::Claude => "missing:claude".to_string(),
        agent::Missing::Adapter => "missing:adapter".to_string(),
        agent::Missing::Agent => "missing:agent".to_string(),
    })?;
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
            if msg != "signin" {
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
    Ok(())
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
    if p.running_session().is_some() {
        return Err("Wait for the running turn to finish.".into());
    }
    let out = turns::undo(&p, &turn_id, &revert_anyway)
        .await
        .map_err(|e| format!("{e:#}"))?;
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
    if p.running_session().is_some() {
        return Err("Wait for the running turn to finish.".into());
    }
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
) -> Res<turns::UndoOutcome> {
    let p = project(&state, &project_id)?;
    turns::revert_file(&p, &turn_id, &path)
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
