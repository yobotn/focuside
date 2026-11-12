//! FocusIDE desktop shell: Tauri commands and events on top of `focuside-core`.

mod commands;
mod files;
mod project;
mod pty;
mod session;
mod turns;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub git: Option<PathBuf>,
    pub projects: Mutex<HashMap<String, Arc<project::Project>>>,
    pub sessions: Mutex<HashMap<String, Arc<session::Session>>>,
    pub ptys: Mutex<HashMap<String, pty::Pty>>,
    /// Folders passed on the command line (`focuside .`) before the UI was ready.
    pub startup_paths: Mutex<Vec<String>>,
}

/// Folder arguments from a command line, resolved against `cwd`.
fn folder_args(argv: &[String], cwd: &str) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .map(|a| {
            let p = PathBuf::from(a);
            let p = if p.is_absolute() {
                p
            } else {
                PathBuf::from(cwd).join(p)
            };
            dunce(p.canonicalize().unwrap_or(p))
        })
        .filter(|p| PathBuf::from(p).is_dir())
        .collect()
}

/// Canonical paths on Windows start with \\?\. The UI and the agent want the plain form.
pub fn dunce(p: PathBuf) -> String {
    let s = p.to_string_lossy().to_string();
    s.strip_prefix(r"\\?\").map(String::from).unwrap_or(s)
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    focuside_core::which::adopt_login_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let paths = folder_args(&argv, &cwd);
            let _ = app.emit("open-paths", paths);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&config_dir)?;
            // The claude-agent-acp adapter is downloaded here on first use.
            focuside_core::adapter_download::set_data_root(data_dir.clone());
            let argv: Vec<String> = std::env::args().collect();
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            app.manage(AppState {
                data_dir,
                config_dir,
                git: focuside_core::which::find("git"),
                projects: Mutex::default(),
                sessions: Mutex::default(),
                ptys: Mutex::default(),
                startup_paths: Mutex::new(folder_args(&argv, &cwd)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::debug_log,
            commands::env_check,
            commands::agent_list,
            commands::adapter_download,
            commands::startup_paths,
            commands::ui_state_load,
            commands::ui_state_save,
            commands::project_open,
            commands::project_close,
            commands::session_start,
            commands::session_prompt,
            commands::session_answer,
            commands::session_run_plan,
            commands::session_discard_plan,
            commands::session_revise_plan,
            commands::session_cancel,
            commands::session_resume,
            commands::session_close,
            commands::session_set_config,
            commands::session_list,
            commands::turn_undo_preview,
            commands::turn_undo,
            commands::turn_redo,
            commands::turn_revert_file,
            commands::turn_file_versions,
            files::fs_list_dir,
            files::fs_all_files,
            files::fs_read,
            files::fs_write,
            files::fs_search,
            files::fs_image,
            pty::pty_open,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_close,
            pty::default_shell_name,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FocusIDE");
}
