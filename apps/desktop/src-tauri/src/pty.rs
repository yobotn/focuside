//! The terminal drawer: one shell per project in a PTY. Output goes to the UI as `pty` events
//! (base64 bytes); closing kills the shell's whole process tree.

use crate::AppState;
use base64::Engine;
use focuside_core::proc::PidTree;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde_json::json;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    tree: Option<PidTree>,
}

impl Pty {
    fn kill(&self) {
        if let Some(t) = &self.tree {
            t.kill();
        }
        let _ = self.child.lock().unwrap().kill();
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.kill();
    }
}

type Res<T> = Result<T, String>;

/// The default shell: pwsh, then Windows PowerShell on Windows; the login shell elsewhere.
fn default_shell() -> (String, Vec<String>) {
    if cfg!(windows) {
        if let Some(p) = focuside_core::which::find("pwsh") {
            return (p.to_string_lossy().to_string(), vec!["-NoLogo".into()]);
        }
        ("powershell.exe".into(), vec!["-NoLogo".into()])
    } else {
        let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        (sh, vec!["-l".into()])
    }
}

#[tauri::command]
pub fn default_shell_name() -> String {
    let (s, _) = default_shell();
    std::path::Path::new(&s)
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(s)
}

#[tauri::command]
pub fn pty_open(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    cols: u16,
    rows: u16,
    shell: Option<String>,
) -> Res<String> {
    let root = state
        .projects
        .lock()
        .unwrap()
        .get(&project_id)
        .map(|p| p.root.clone())
        .ok_or("This project is not open.")?;
    let (prog, args) = match shell.filter(|s| !s.trim().is_empty()) {
        Some(s) => (s, vec![]),
        None => default_shell(),
    };
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(10),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;
    let mut cmd = CommandBuilder::new(&prog);
    cmd.args(&args);
    cmd.cwd(&root);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let tree = child.process_id().map(PidTree::attach);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    static N: AtomicU64 = AtomicU64::new(1);
    let id = format!("pty{}", N.fetch_add(1, Ordering::Relaxed));
    let pty = Pty {
        master: pair.master,
        writer: Mutex::new(writer),
        child: Mutex::new(child),
        tree,
    };
    state.ptys.lock().unwrap().insert(id.clone(), pty);

    let id2 = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        let b64 = base64::engine::general_purpose::STANDARD;
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = app.emit("pty", json!({ "id": id2, "data": b64.encode(&buf[..n]) }));
                }
            }
        }
        let _ = app.emit("pty-exit", json!({ "id": id2 }));
    });
    Ok(id)
}

#[tauri::command]
pub fn pty_write(state: State<'_, AppState>, id: String, data: String) -> Res<()> {
    let ptys = state.ptys.lock().unwrap();
    let p = ptys.get(&id).ok_or("The terminal is closed.")?;
    let mut w = p.writer.lock().unwrap();
    w.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, AppState>, id: String, cols: u16, rows: u16) -> Res<()> {
    let ptys = state.ptys.lock().unwrap();
    let p = ptys.get(&id).ok_or("The terminal is closed.")?;
    p.master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(10),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_close(state: State<'_, AppState>, id: String) -> Res<()> {
    let p = state.ptys.lock().unwrap().remove(&id);
    if let Some(p) = p {
        p.kill();
    }
    Ok(())
}
