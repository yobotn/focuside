//! File commands for the tree, viewer, pickers and search, plus the project watcher.

use crate::project::Project;
use crate::AppState;
use focuside_core::files::{self, DirEntry, FileContent, FileMatches, SearchQuery, SearchResult};
use focuside_core::replace;
use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    tauri::async_runtime::spawn_blocking(move || files::list_dir(&p.root, &path, show_ignored))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn fs_all_files(state: State<'_, AppState>, project_id: String) -> Res<Vec<String>> {
    let p = project(&state, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || files::all_files(&p.root))
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
    tauri::async_runtime::spawn_blocking(move || files::read_file(&p.root, &path))
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
    note_user_saves(&state, &p, &[path]);
    Ok(())
}

/// Saves by the user during a running turn count as "your edit", which Undo turn keeps.
fn note_user_saves(state: &AppState, p: &Project, paths: &[String]) {
    let Some(key) = p.running_session() else {
        return;
    };
    let s = state.sessions.lock().unwrap().get(&key).cloned();
    let pane = state.panes.lock().unwrap().get(&key).cloned();
    for path in paths {
        let rel = path.replace('\\', "/");
        if let Some(s) = &s {
            s.note_user_save(&rel);
        }
        if let Some(pane) = &pane {
            pane.note_user_save(&rel);
        }
    }
}

#[tauri::command]
pub async fn fs_search(
    state: State<'_, AppState>,
    project_id: String,
    query: SearchQuery,
) -> Res<SearchResult> {
    let p = project(&state, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || files::search(&p.root, &query, 2000))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

/// The files Replace all would change, with the number of matches in each.
#[tauri::command]
pub async fn fs_replace_preview(
    state: State<'_, AppState>,
    project_id: String,
    query: SearchQuery,
) -> Res<Vec<FileMatches>> {
    let p = project(&state, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || files::count_matches(&p.root, &query, 5000))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

/// Replace all batches of this run, for Undo. Only the newest few are kept.
static REPLACES: Mutex<Vec<(String, String, replace::Batch)>> = Mutex::new(Vec::new());
static REPLACE_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceReply {
    /// Pass to `fs_replace_undo`. None when Undo is not possible.
    id: Option<String>,
    #[serde(flatten)]
    outcome: replace::ReplaceOutcome,
}

/// Replaces every match in the given files (from `fs_replace_preview`, after the user
/// confirmed). A snapshot taken first makes Undo possible.
#[tauri::command]
pub async fn fs_replace(
    state: State<'_, AppState>,
    project_id: String,
    query: SearchQuery,
    replace: String,
    paths: Vec<String>,
) -> Res<ReplaceReply> {
    let p = project(&state, &project_id)?;
    let id = format!(
        "{}-{}",
        crate::turns::now(),
        REPLACE_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let (batch, outcome) =
        replace::replace_in_files(&p.root, &p.shadow, &id, &query, &replace, &paths)
            .await
            .map_err(|e| format!("{e:#}"))?;
    note_user_saves(&state, &p, &batch.paths());
    let id = outcome.undo.then(|| {
        let mut list = REPLACES.lock().unwrap();
        list.push((id.clone(), project_id.clone(), batch));
        let extra = list.len().saturating_sub(10);
        list.drain(..extra);
        id
    });
    Ok(ReplaceReply { id, outcome })
}

/// Undo of one Replace all: files that changed since then stay as they are.
#[tauri::command]
pub async fn fs_replace_undo(
    state: State<'_, AppState>,
    project_id: String,
    id: String,
) -> Res<replace::UndoReplaceOutcome> {
    let p = project(&state, &project_id)?;
    let batch = {
        let mut list = REPLACES.lock().unwrap();
        let i = list
            .iter()
            .position(|(x, pid, _)| *x == id && *pid == project_id)
            .ok_or_else(|| "This replace can no longer be undone.".to_string())?;
        list.remove(i).2
    };
    let out = replace::undo_replace(&p.root, &p.shadow, &batch)
        .await
        .map_err(|e| format!("{e:#}"))?;
    note_user_saves(&state, &p, &out.restored);
    Ok(out)
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
                    // The plan folder never counts as a turn change (Settings > Plan folder).
                    log.extend(paths.iter().filter(|r| !p.shadow.is_skipped(r)).cloned());
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

/// A project image as a data URL, for images in rendered markdown. Only common image types
/// up to 8 MB; the path must stay inside the project.
#[tauri::command]
pub async fn fs_image(state: State<'_, AppState>, project_id: String, path: String) -> Res<String> {
    use base64::Engine;
    let p = project(&state, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let full = files::resolve(&p.root, &path).map_err(|e| format!("{e:#}"))?;
        let mime = match files::media_mime(&path) {
            Some(m) if m.starts_with("image/") => m,
            _ => return Err("Not an image.".to_string()),
        };
        let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
        if meta.len() > 8 * 1024 * 1024 {
            return Err("Image is too large.".to_string());
        }
        let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(format!("data:{mime};base64,{b64}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// An image or PDF inside the project as raw bytes, for the viewer. Images up to 50 MB,
/// PDFs up to 100 MB. Raw bytes avoid base64 over IPC and need no asset protocol.
#[tauri::command]
pub async fn fs_media(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Res<tauri::ipc::Response> {
    let p = project(&state, &project_id)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || files::read_media(&p.root, &path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))?;
    Ok(tauri::ipc::Response::new(bytes))
}
