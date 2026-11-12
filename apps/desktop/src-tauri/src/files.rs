//! File commands for the tree, viewer, pickers and search, plus the project watcher.

use crate::project::Project;
use crate::AppState;
use focuside_core::files::{self, DirEntry, FileContent, SearchResult};
use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

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

#[tauri::command]
pub async fn fs_list_dir(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    show_ignored: bool,
) -> Res<Vec<DirEntry>> {
    let p = project(&state, &project_id)?;
    tokio::task::spawn_blocking(move || files::list_dir(&p.root, &path, show_ignored))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn fs_all_files(state: State<'_, AppState>, project_id: String) -> Res<Vec<String>> {
    let p = project(&state, &project_id)?;
    tokio::task::spawn_blocking(move || files::all_files(&p.root))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_read(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Res<FileContent> {
    let p = project(&state, &project_id)?;
    tokio::task::spawn_blocking(move || files::read_file(&p.root, &path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

/// Saves a file from the viewer. During a running turn the save is recorded as "your edit",
/// which Undo turn keeps by default.
#[tauri::command]
pub async fn fs_write(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    content: String,
) -> Res<()> {
    let p = project(&state, &project_id)?;
    files::write_file(&p.root, &path, &content).map_err(|e| format!("{e:#}"))?;
    if let Some(key) = p.running_session() {
        let s = state.sessions.lock().unwrap().get(&key).cloned();
        if let Some(s) = s {
            s.note_user_save(&path.replace('\\', "/"));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn fs_search(
    state: State<'_, AppState>,
    project_id: String,
    query: String,
    case_sensitive: bool,
    regex: bool,
) -> Res<SearchResult> {
    let p = project(&state, &project_id)?;
    tokio::task::spawn_blocking(move || files::search(&p.root, &query, case_sensitive, regex, 2000))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

/// Watches the project folder. Changes go to the UI as `fs-changed` events and, during a
/// running turn, into the turn's watch log (used to find ignored files that changed).
pub fn watch(app: AppHandle, project: &Arc<Project>) {
    let root = project.root.clone();
    let id = project.id.clone();
    let weak = Arc::downgrade(project);
    let debouncer = new_debouncer(
        Duration::from_millis(250),
        move |res: DebounceEventResult| {
            let Ok(events) = res else { return };
            let mut paths = BTreeSet::new();
            for ev in events {
                let Ok(rel) = ev.path.strip_prefix(&root) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                if rel.is_empty() || rel == ".git" || rel.starts_with(".git/") {
                    continue;
                }
                paths.insert(rel);
            }
            if paths.is_empty() {
                return;
            }
            if let Some(p) = weak.upgrade() {
                if let Some(log) = p.watch_log.lock().unwrap().as_mut() {
                    log.extend(paths.iter().cloned());
                }
            }
            let _ = app.emit("fs-changed", json!({ "projectId": id, "paths": paths }));
        },
    );
    match debouncer {
        Ok(mut d) => {
            if let Err(e) = d.watcher().watch(&project.root, RecursiveMode::Recursive) {
                tracing::warn!("watch {}: {e}", project.root.display());
            }
            *project.watcher.lock().unwrap() = Some(Box::new(d));
        }
        Err(e) => tracing::warn!("watcher: {e}"),
    }
}
