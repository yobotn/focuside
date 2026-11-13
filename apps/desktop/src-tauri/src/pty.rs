//! Programs in a PTY: the terminal drawer (one shell per project) and agent panes (the agent's
//! own TUI, one per pane session). Output goes to the UI as `pty` events (base64 bytes); exit
//! as `pty-exit`. Closing kills the program's whole process tree.
//!
//! FocusIDE writes into a PTY only what the user types or pastes (`pty_write` from the
//! terminal's own input). It never sends prompts, commands or slash commands into an agent
//! pane. The drawer's "Run in terminal" (install commands) goes to the shell only.

use crate::AppState;
use base64::Engine;
use focuside_core::proc::PidTree;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde_json::json;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    tree: Option<PidTree>,
    /// The pane session this PTY belongs to. None for the terminal drawer.
    pub session: Option<String>,
}

impl Pty {
    fn kill(&self) {
        if let Some(t) = &self.tree {
            t.kill();
        }
        let _ = self.killer.lock().unwrap().kill();
    }

    pub fn write(&self, data: &[u8]) -> std::io::Result<()> {
        let mut w = self.writer.lock().unwrap();
        w.write_all(data)?;
        w.flush()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.kill();
    }
}

/// What to start in a PTY. `env` adds to the app's environment; nothing is removed.
#[derive(Debug, Clone)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Variables every PTY gets, so TUIs pick colors and keys that xterm.js understands. They are
/// the only variables FocusIDE adds; auth variables are never added or removed.
const TERM_ENV: &[(&str, &str)] = &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")];

/// Starts `launch` in a new PTY (ConPTY on Windows) in `cwd`. `on_data` gets the output.
/// `on_exit` runs once, after the program exited and its last output was read.
pub fn spawn(
    launch: &Launch,
    cwd: &Path,
    cols: u16,
    rows: u16,
    session: Option<String>,
    mut on_data: impl FnMut(&[u8]) + Send + 'static,
    on_exit: impl FnOnce() + Send + 'static,
) -> anyhow::Result<Pty> {
    let pair = native_pty_system().openpty(PtySize {
        rows: rows.max(2),
        cols: cols.max(10),
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut cmd = CommandBuilder::new(&launch.program);
    cmd.args(&launch.args);
    cmd.cwd(cwd);
    for (k, v) in TERM_ENV {
        cmd.env(k, v);
    }
    for (k, v) in &launch.env {
        cmd.env(k, v);
    }
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);
    let tree = child.process_id().map(PidTree::attach);
    let killer = child.clone_killer();
    let mut reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => on_data(&buf[..n]),
            }
        }
        let _ = done_tx.send(());
    });
    // ConPTY keeps the output pipe open after the program exits, until the pseudo console is
    // closed. So exit is taken from the process, and the reader gets a short time to drain.
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = done_rx.recv_timeout(Duration::from_millis(400));
        on_exit();
    });

    Ok(Pty {
        master: pair.master,
        writer: Mutex::new(writer),
        killer: Mutex::new(killer),
        tree,
        session,
    })
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

/// Opens a PTY in the project folder. Without `session` it starts the shell (the terminal
/// drawer). With `session` it starts that pane session's agent TUI, with the program and
/// arguments the pane resolved in Rust; an earlier PTY of the same pane is closed first.
#[tauri::command]
pub fn pty_open(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    cols: u16,
    rows: u16,
    shell: Option<String>,
    session: Option<String>,
) -> Res<String> {
    let root = state
        .projects
        .lock()
        .unwrap()
        .get(&project_id)
        .map(|p| p.root.clone())
        .ok_or("This project is not open.")?;
    let pane = match &session {
        Some(key) => {
            let pane = state
                .panes
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or("This session is closed.")?;
            if pane.project.id != project_id {
                return Err("This session is closed.".into());
            }
            Some(pane)
        }
        None => None,
    };
    let launch = match &pane {
        Some(p) => p.launch.clone(),
        None => {
            let (program, args) = match shell.filter(|s| !s.trim().is_empty()) {
                Some(s) => (s, vec![]),
                None => default_shell(),
            };
            Launch {
                program,
                args,
                env: vec![],
            }
        }
    };
    static N: AtomicU64 = AtomicU64::new(1);
    let id = format!("pty{}", N.fetch_add(1, Ordering::Relaxed));
    if let Some(p) = &pane {
        // From here, exit events of an earlier PTY of this pane are ignored.
        p.set_pty(Some(id.clone()));
    }
    if let Some(key) = &session {
        close_session(&state, key);
    }
    let b64 = base64::engine::general_purpose::STANDARD;
    let (app2, id2) = (app.clone(), id.clone());
    let on_data = move |bytes: &[u8]| {
        let _ = app2.emit("pty", json!({ "id": id2, "data": b64.encode(bytes) }));
    };
    let (app3, id3, pane3) = (app.clone(), id.clone(), pane.clone());
    let on_exit = move || {
        // Dropping the PTY closes the pseudo console and ends the reader.
        let removed = app3.state::<AppState>().ptys.lock().unwrap().remove(&id3);
        drop(removed);
        let _ = app3.emit("pty-exit", json!({ "id": id3 }));
        if let Some(p) = pane3 {
            let id = id3.clone();
            tauri::async_runtime::spawn(async move { p.on_pty_exit(&id).await });
        }
    };
    let pty = match spawn(&launch, &root, cols, rows, session, on_data, on_exit) {
        Ok(p) => p,
        Err(e) => {
            if let Some(p) = &pane {
                p.set_pty(None);
            }
            return Err(format!("{e:#}"));
        }
    };
    state.ptys.lock().unwrap().insert(id.clone(), pty);
    if let Some(p) = pane {
        tauri::async_runtime::spawn(async move { p.on_pty_open().await });
    }
    Ok(id)
}

/// Closes every PTY of a pane session (its agent TUI and the processes it started).
pub fn close_session(state: &AppState, key: &str) {
    let removed: Vec<Pty> = {
        let mut ptys = state.ptys.lock().unwrap();
        let ids: Vec<String> = ptys
            .iter()
            .filter(|(_, p)| p.session.as_deref() == Some(key))
            .map(|(id, _)| id.clone())
            .collect();
        ids.into_iter().filter_map(|id| ptys.remove(&id)).collect()
    };
    for p in removed {
        p.kill();
    }
}

/// Sends the user's keystrokes or paste from the terminal view. This is the only way data
/// gets into a PTY.
#[tauri::command]
pub fn pty_write(state: State<'_, AppState>, id: String, data: String) -> Res<()> {
    let ptys = state.ptys.lock().unwrap();
    let p = ptys.get(&id).ok_or("The terminal is closed.")?;
    p.write(data.as_bytes()).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    enum Ev {
        Data,
        Exit,
    }

    /// Runs `launch` through `spawn` and returns its output once it exits. Like xterm.js in
    /// the app, it answers the cursor position request (ESC [6n) that ConPTY sends at start.
    fn run(launch: Launch) -> String {
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        let out2 = out.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let tx2 = tx.clone();
        let dir = std::env::temp_dir();
        let pty = spawn(
            &launch,
            &dir,
            100,
            30,
            Some("s1".into()),
            move |b| {
                out2.lock().unwrap().extend_from_slice(b);
                let _ = tx2.send(Ev::Data);
            },
            move || {
                let _ = tx.send(Ev::Exit);
            },
        )
        .expect("spawn");
        assert_eq!(pty.session.as_deref(), Some("s1"));
        let mut answered = 0;
        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ev::Exit) => break,
                Ok(Ev::Data) => {
                    let asks = String::from_utf8_lossy(&out.lock().unwrap())
                        .matches("\x1b[6n")
                        .count();
                    while answered < asks {
                        pty.write(b"\x1b[1;1R").unwrap();
                        answered += 1;
                    }
                }
                Err(_) => panic!(
                    "the program did not exit; output: {:?}",
                    out.lock().unwrap()
                ),
            }
        }
        drop(pty);
        let text = String::from_utf8_lossy(&out.lock().unwrap()).to_string();
        text
    }

    /// The pane launch path: a program with arguments and an extra variable, output read
    /// through the PTY (ConPTY on Windows), exit reported.
    #[test]
    fn runs_a_program_and_reports_exit() {
        let launch = if cfg!(windows) {
            Launch {
                program: "cmd.exe".into(),
                args: vec!["/c".into(), "echo pty-%FOCUSIDE_PTY_TEST%".into()],
                env: vec![("FOCUSIDE_PTY_TEST".into(), "ok".into())],
            }
        } else {
            Launch {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), "echo pty-$FOCUSIDE_PTY_TEST".into()],
                env: vec![("FOCUSIDE_PTY_TEST".into(), "ok".into())],
            }
        };
        let text = run(launch);
        assert!(text.contains("pty-ok"), "output was: {text:?}");
    }

    /// Starts the real `claude --version` the way a Claude pane starts `claude`:
    /// `cargo test -p focuside-desktop -- --ignored real_claude_in_pty`.
    #[test]
    #[ignore]
    fn real_claude_in_pty() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let mut args = tui.args.clone();
        args.push("--version".into());
        let text = run(Launch {
            program: tui.program.to_string_lossy().to_string(),
            args,
            env: vec![],
        });
        println!("{text}");
        assert!(text.contains("Claude Code"), "output was: {text:?}");
    }

    /// Starts the real `claude` TUI in a PTY, types nothing, checks that it draws a screen, then
    /// closes it the way pane close does and checks that it exits:
    /// `cargo test -p focuside-desktop -- --ignored real_claude_tui_starts`.
    #[test]
    #[ignore]
    fn real_claude_tui_starts_and_closes() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let dir = tempfile::tempdir().unwrap();
        tui_starts_and_closes(
            Launch {
                program: tui.program.to_string_lossy().to_string(),
                args: tui.args.clone(),
                env: vec![],
            },
            dir.path(),
        );
    }

    /// The same with the launch of a Claude pane after the plugin question was answered yes:
    /// `--plugin-dir` with the generated plugin and `FOCUSIDE_EVENTS`. Needs the built helper
    /// (`node scripts/build-hook.mjs`):
    /// `cargo test -p focuside-desktop -- --ignored real_claude_tui_with_plugin`.
    #[test]
    #[ignore]
    fn real_claude_tui_with_plugin() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let data = tempfile::tempdir().unwrap();
        let helper = crate::hooks::helper_path().expect("focuside-hook built");
        let plugin = crate::hooks::ensure_plugin(data.path(), &helper).unwrap();
        let events = data.path().join("events");
        std::fs::create_dir_all(&events).unwrap();
        let launch = crate::pane::launch_for(tui, &events, Some(&plugin));
        assert!(launch.args.contains(&"--plugin-dir".to_string()));
        let dir = tempfile::tempdir().unwrap();
        tui_starts_and_closes(launch, dir.path());
        // No prompt was typed, so no hook ran.
        assert!(crate::pane::take_events(&events).is_empty());
    }

    fn tui_starts_and_closes(launch: Launch, cwd: &std::path::Path) {
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        let out2 = out.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let pty = spawn(
            &launch,
            cwd,
            120,
            40,
            Some("s1".into()),
            move |b| out2.lock().unwrap().extend_from_slice(b),
            move || {
                let _ = tx.send(());
            },
        )
        .expect("spawn");
        // Answer cursor position requests, as xterm.js does, for a few seconds.
        let mut answered = 0;
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(200));
            let asks = String::from_utf8_lossy(&out.lock().unwrap())
                .matches("\x1b[6n")
                .count();
            while answered < asks {
                pty.write(b"\x1b[1;1R").unwrap();
                answered += 1;
            }
        }
        let bytes = out.lock().unwrap().len();
        println!("claude TUI wrote {bytes} bytes");
        assert!(bytes > 200, "the TUI drew a screen");
        assert!(rx.try_recv().is_err(), "the TUI still runs");
        drop(pty);
        rx.recv_timeout(Duration::from_secs(10))
            .expect("closing the PTY ends the TUI");
    }
}
