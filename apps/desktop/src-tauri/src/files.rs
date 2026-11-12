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
    if let Some(key) = p.running_session() {
        let rel = path.replace('\\', "/");
        let s = state.sessions.lock().unwrap().get(&key).cloned();
        if let Some(s) = s {
            s.note_user_save(&rel);
        }
        let pane = state.panes.lock().unwrap().get(&key).cloned();
        if let Some(pane) = pane {
            pane.note_user_save(&rel);
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
    tauri::async_runtime::spawn_blocking(move || {
        files::search(&p.root, &query, case_sensitive, regex, 2000)
    })
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

/// A project image as a data URL, for images in rendered markdown. Only common image types
/// up to 8 MB; the path must stay inside the project.
#[tauri::command]
pub async fn fs_image(state: State<'_, AppState>, project_id: String, path: String) -> Res<String> {
    use base64::Engine;
    let p = project(&state, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let full = files::resolve(&p.root, &path).map_err(|e| format!("{e:#}"))?;
        let ext = full
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let mime = match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "avif" => "image/avif",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
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
