//! Git panel commands on the project's own repository (`focuside_core::git`). They never touch
//! the shadow repo of the snapshots. Each command runs git on a blocking thread.
//!
//! Fetch, pull and push use the user's own git setup and credential helper. FocusIDE never
//! reads, stores or asks for credentials; git's error text goes to the UI as it is.

use crate::project::Project;
use crate::AppState;
use focuside_core::git::{Branch, Commit, CommitFile, DiffSource, Repo, Status, Versions};
use std::sync::Arc;
use tauri::State;

type Res<T> = Result<T, String>;

fn project(state: &AppState, id: &str) -> Res<Arc<Project>> {
    state
        .projects
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| "This project is not open.".to_string())
}

/// Runs `f` with the project's repository on a blocking thread.
async fn with_repo<T, F>(state: &AppState, project_id: &str, f: F) -> Res<T>
where
    T: Send + 'static,
    F: FnOnce(&Repo) -> Res<T> + Send + 'static,
{
    let p = project(state, project_id)?;
    let git = state
        .git
        .clone()
        .ok_or_else(|| "Git is not installed.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || f(&Repo::new(&git, &p.root)))
        .await
        .map_err(|e| e.to_string())?
}

/// Switching branches and pulling rewrite many files. While an agent turn runs here, they
/// would mix with the agent's changes and Undo, so they wait.
fn not_running(state: &AppState, project_id: &str) -> Res<()> {
    if project(state, project_id)?.running_session().is_some() {
        return Err("A prompt is running in this project. Try again when it is done.".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>, project_id: String) -> Res<Status> {
    with_repo(&state, &project_id, |r| r.status()).await
}

#[tauri::command]
pub async fn git_init(state: State<'_, AppState>, project_id: String) -> Res<()> {
    with_repo(&state, &project_id, |r| r.init()).await
}

#[tauri::command]
pub async fn git_stage(
    state: State<'_, AppState>,
    project_id: String,
    paths: Vec<String>,
) -> Res<()> {
    with_repo(&state, &project_id, move |r| r.stage(&paths)).await
}

#[tauri::command]
pub async fn git_stage_all(state: State<'_, AppState>, project_id: String) -> Res<()> {
    with_repo(&state, &project_id, |r| r.stage_all()).await
}

#[tauri::command]
pub async fn git_unstage(
    state: State<'_, AppState>,
    project_id: String,
    paths: Vec<String>,
) -> Res<()> {
    with_repo(&state, &project_id, move |r| r.unstage(&paths)).await
}

#[tauri::command]
pub async fn git_unstage_all(state: State<'_, AppState>, project_id: String) -> Res<()> {
    with_repo(&state, &project_id, |r| r.unstage_all()).await
}

/// The UI asks the user first.
#[tauri::command]
pub async fn git_discard(
    state: State<'_, AppState>,
    project_id: String,
    tracked: Vec<String>,
    untracked: Vec<String>,
) -> Res<()> {
    with_repo(&state, &project_id, move |r| {
        r.discard(&tracked, &untracked)
    })
    .await
}

#[tauri::command]
pub async fn git_commit(
    state: State<'_, AppState>,
    project_id: String,
    message: String,
    all: bool,
) -> Res<String> {
    with_repo(&state, &project_id, move |r| r.commit(&message, all)).await
}

#[tauri::command]
pub async fn git_branches(state: State<'_, AppState>, project_id: String) -> Res<Vec<Branch>> {
    with_repo(&state, &project_id, |r| r.branches()).await
}

#[tauri::command]
pub async fn git_switch(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    remote: bool,
) -> Res<()> {
    not_running(&state, &project_id)?;
    with_repo(&state, &project_id, move |r| r.switch(&name, remote)).await
}

#[tauri::command]
pub async fn git_create_branch(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Res<()> {
    with_repo(&state, &project_id, move |r| r.create_branch(&name)).await
}

#[tauri::command]
pub async fn git_fetch(state: State<'_, AppState>, project_id: String) -> Res<()> {
    with_repo(&state, &project_id, |r| r.fetch()).await
}

#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>, project_id: String) -> Res<()> {
    not_running(&state, &project_id)?;
    with_repo(&state, &project_id, |r| r.pull()).await
}

#[tauri::command]
pub async fn git_push(
    state: State<'_, AppState>,
    project_id: String,
    has_upstream: bool,
) -> Res<()> {
    with_repo(&state, &project_id, move |r| r.push(has_upstream)).await
}

#[tauri::command]
pub async fn git_log(
    state: State<'_, AppState>,
    project_id: String,
    limit: usize,
) -> Res<Vec<Commit>> {
    with_repo(&state, &project_id, move |r| r.log(limit.clamp(1, 500))).await
}

#[tauri::command]
pub async fn git_commit_files(
    state: State<'_, AppState>,
    project_id: String,
    hash: String,
) -> Res<Vec<CommitFile>> {
    with_repo(&state, &project_id, move |r| r.commit_files(&hash)).await
}

/// Both sides of one file of a change. `path` is relative to the repository root.
#[tauri::command]
pub async fn git_versions(
    state: State<'_, AppState>,
    project_id: String,
    source: DiffSource,
    path: String,
    orig: Option<String>,
) -> Res<Versions> {
    with_repo(&state, &project_id, move |r| {
        Ok(r.versions(&source, &path, orig.as_deref()))
    })
    .await
}

/// The HEAD version of a file for the change markers in the editor. `path` is relative to the
/// repository root. None when the file is not in HEAD or is binary.
#[tauri::command]
pub async fn git_head_text(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Res<Option<String>> {
    with_repo(&state, &project_id, move |r| Ok(r.head_text(&path))).await
}
