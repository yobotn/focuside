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
/// `on_exit` runs once, after the program exited and its last output was read, with the exit
/// code when the process reported one.
pub fn spawn(
    launch: &Launch,
    cwd: &Path,
    cols: u16,
    rows: u16,
    session: Option<String>,
    mut on_data: impl FnMut(&[u8]) + Send + 'static,
    on_exit: impl FnOnce(Option<u32>) + Send + 'static,
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
        let code = child.wait().ok().map(|s| s.exit_code());
        let _ = done_rx.recv_timeout(Duration::from_millis(400));
        on_exit(code);
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
        Some(p) => p.launch(),
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
    let on_exit = move |code: Option<u32>| {
        // Dropping the PTY closes the pseudo console and ends the reader.
        let removed = app3.state::<AppState>().ptys.lock().unwrap().remove(&id3);
        drop(removed);
        let _ = app3.emit("pty-exit", json!({ "id": id3 }));
        if let Some(p) = pane3 {
            let id = id3.clone();
            tauri::async_runtime::spawn(async move { p.on_pty_exit(&id, code).await });
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
            move |_| {
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

    /// The same with the launch of a Claude pane with "Plan first" on: `--permission-mode plan`.
    /// `cargo test -p focuside-desktop -- --ignored real_claude_tui_in_plan_mode`.
    #[test]
    #[ignore]
    fn real_claude_tui_in_plan_mode() {
        let mut tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        assert!(tui.start_in_plan_mode("claude"));
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

    /// The same with the launch of a Claude pane with turn tracking on (the default):
    /// `--plugin-dir` with the generated plugin and `FOCUSIDE_EVENTS`. Needs the built helper
    /// (`node scripts/build-hook.mjs`):
    /// `cargo test -p focuside-desktop -- --ignored real_claude_tui_with_plugin`.
    #[test]
    #[ignore]
    fn real_claude_tui_with_plugin() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let data = tempfile::tempdir().unwrap();
        let helper = crate::hooks::helper_path().expect("focuside-hook built");
        let plugin = crate::hooks::ensure_plugin(data.path(), &helper, false).unwrap();
        let events = data.path().join("events");
        std::fs::create_dir_all(&events).unwrap();
        let launch = crate::pane::launch_for(tui, &events, Some(&plugin));
        assert!(launch.args.contains(&"--plugin-dir".to_string()));
        let dir = tempfile::tempdir().unwrap();
        tui_starts_and_closes(launch, dir.path());
        // No prompt was typed, so no hook ran.
        assert!(crate::pane::take_events(&events).is_empty());
    }

    /// Status line texts of Claude Code's permission modes (Claude Code 2.1.x, permission
    /// modes docs) and the mode names hooks report.
    const MODES: &[(&str, &str)] = &[
        ("plan mode on", "plan"),
        ("accept edits on", "acceptEdits"),
        ("auto mode on", "auto"),
        ("manual mode on", "default"),
        ("bypass permissions on", "bypassPermissions"),
    ];

    /// The mode that the Claude TUI's status line shows, read from the bottom lines of the
    /// rendered screen.
    fn shown_mode(screen: &str) -> Option<&'static str> {
        // The lowest status line wins: the classic renderer can leave older ones above it.
        screen.lines().rev().take(8).find_map(|line| {
            MODES
                .iter()
                .find(|(s, _)| line.contains(s))
                .map(|(_, m)| *m)
        })
    }

    /// The real Claude TUI in a PTY, drawn into a vt100 screen. Cursor position requests are
    /// answered as xterm.js does. Runs in this repo, a folder Claude already trusts, so no
    /// trust question shows.
    struct ClaudeScreen {
        pty: Pty,
        screen: Arc<Mutex<vt100::Parser>>,
        asks: Arc<std::sync::atomic::AtomicUsize>,
        answered: usize,
        /// Every byte the TUI wrote, for the terminal title sequences.
        raw: Arc<Mutex<Vec<u8>>>,
        /// Set when the TUI exited: its exit code.
        exit: Arc<Mutex<Option<Option<u32>>>>,
    }

    impl ClaudeScreen {
        fn start(launch: Launch) -> Self {
            let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
            Self::start_in(launch, &cwd)
        }

        fn start_in(launch: Launch, cwd: &Path) -> Self {
            let screen = Arc::new(Mutex::new(vt100::Parser::new(40, 120, 0)));
            let asks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let (screen2, asks2) = (screen.clone(), asks.clone());
            let raw = Arc::new(Mutex::new(Vec::new()));
            let raw2 = raw.clone();
            let exit = Arc::new(Mutex::new(None));
            let exit2 = exit.clone();
            let pty = spawn(
                &launch,
                cwd,
                120,
                40,
                None,
                move |b| {
                    let n = String::from_utf8_lossy(b).matches("\x1b[6n").count();
                    asks2.fetch_add(n, Ordering::Relaxed);
                    raw2.lock().unwrap().extend_from_slice(b);
                    screen2.lock().unwrap().process(b);
                },
                move |code| *exit2.lock().unwrap() = Some(code),
            )
            .expect("spawn");
            Self {
                pty,
                screen,
                asks,
                answered: 0,
                raw,
                exit,
            }
        }

        /// The exit code once the TUI exited: Some(code), else None while it runs.
        fn exited(&self) -> Option<Option<u32>> {
            *self.exit.lock().unwrap()
        }

        /// The terminal titles the TUI set (OSC 0 and OSC 2), in order.
        fn titles(&self) -> Vec<String> {
            let raw = String::from_utf8_lossy(&self.raw.lock().unwrap()).to_string();
            let mut out = vec![];
            let mut rest = raw.as_str();
            while let Some(i) = rest.find("\x1b]") {
                rest = &rest[i + 2..];
                let end = rest.find(['\x07', '\x1b']).unwrap_or(rest.len());
                let body = &rest[..end];
                if let Some(t) = body.strip_prefix("0;").or_else(|| body.strip_prefix("2;")) {
                    out.push(t.to_string());
                }
                rest = &rest[end..];
            }
            out
        }

        /// The last `n` lines of the screen that are not empty.
        fn bottom(&self, n: usize) -> Vec<String> {
            let text = self.screen.lock().unwrap().screen().contents();
            let lines: Vec<String> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.trim_end().to_string())
                .collect();
            lines[lines.len().saturating_sub(n)..].to_vec()
        }

        /// Waits until the screen shows `text`, at most `ms`. Returns the screen text.
        fn wait_for(&mut self, text: &str, ms: u64) -> String {
            let mut screen = String::new();
            for _ in 0..ms / 200 {
                screen = self.wait(200);
                if screen.contains(text) {
                    break;
                }
            }
            screen
        }

        /// Waits `ms`, then returns the screen text.
        fn wait(&mut self, ms: u64) -> String {
            for _ in 0..ms / 100 {
                std::thread::sleep(Duration::from_millis(100));
                while self.answered < self.asks.load(Ordering::Relaxed) {
                    self.pty.write(b"\x1b[1;1R").unwrap();
                    self.answered += 1;
                }
            }
            self.screen.lock().unwrap().screen().contents()
        }
    }

    /// The pane's mode button on the real Claude TUI: starts `claude --permission-mode plan`,
    /// sends the Shift+Tab key sequence (ESC [Z) four times and reads the mode from the
    /// rendered status line after each press. Only the mode key is sent: no prompt, no Enter.
    /// `cargo test -p focuside-desktop -- --ignored real_claude_mode_key --nocapture`.
    #[test]
    #[ignore]
    fn real_claude_mode_key() {
        let mut tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        tui.start_in_plan_mode("claude");
        let mut c = ClaudeScreen::start(Launch {
            program: tui.program.to_string_lossy().to_string(),
            args: tui.args.clone(),
            env: vec![],
        });
        c.wait_for("shift+tab to cycle", 30000);
        // The status line shows before the TUI reads keys.
        let first = shown_mode(&c.wait(2000));
        println!("at start: {first:?}");
        assert_eq!(first, Some("plan"), "started in plan mode");
        let mut seen = vec![];
        for _ in 0..4 {
            c.pty.write(crate::pane::MODE_KEY).unwrap();
            let m = shown_mode(&c.wait(1500));
            println!("after Shift+Tab: {m:?}");
            seen.push(m);
        }
        assert!(
            seen[0].is_some_and(|m| m != "plan"),
            "Shift+Tab left plan mode: {seen:?}"
        );
        assert!(
            seen.contains(&Some("plan")),
            "the cycle came back to plan mode: {seen:?}"
        );
    }

    /// The strings the pane reads from the real Claude TUI: the status line of each permission
    /// mode and the terminal titles. Starts `claude` in its default mode, prints the bottom lines
    /// after each Shift+Tab (ESC [Z, at most 5) and the OSC 0 and OSC 2 titles. Only the mode key
    /// is sent: no prompt, no Enter.
    /// `cargo test -p focuside-desktop -- --ignored real_claude_mode_strings --nocapture`.
    #[test]
    #[ignore]
    fn real_claude_mode_strings_and_titles() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let mut c = ClaudeScreen::start(Launch {
            program: tui.program.to_string_lossy().to_string(),
            args: tui.args.clone(),
            env: vec![],
        });
        c.wait_for("shift+tab to cycle", 30000);
        c.wait(2000);
        println!("at start: {:?}", c.bottom(3));
        let mut seen = vec![shown_mode(&c.wait(100))];
        for _ in 0..5 {
            c.pty.write(crate::pane::MODE_KEY).unwrap();
            c.wait(1500);
            println!("after Shift+Tab: {:?}", c.bottom(3));
            seen.push(shown_mode(&c.wait(100)));
        }
        println!("modes: {seen:?}");
        println!("titles: {:?}", c.titles());
        assert!(seen.contains(&Some("plan")), "plan mode shows: {seen:?}");
    }

    /// Settings > Plan folder: starts the Claude TUI with `--settings {"plansDirectory":...}`
    /// (the JSON goes through the Windows command line), checks that no settings error shows,
    /// and opens the local `/status` view (no model call) to print the setting sources. Only
    /// the command and Esc are typed, no prompt.
    /// `cargo test -p focuside-desktop -- --ignored real_claude_plan_folder --nocapture`.
    #[test]
    #[ignore]
    fn real_claude_plan_folder() {
        let mut tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        assert!(tui.set_plans_folder("claude", "target/plans-test"));
        println!("args: {:?}", tui.args);
        let mut c = ClaudeScreen::start(Launch {
            program: tui.program.to_string_lossy().to_string(),
            args: tui.args.clone(),
            env: vec![],
        });
        let start = c.wait_for("shift+tab to cycle", 30000);
        assert!(
            start.contains("shift+tab to cycle"),
            "the TUI started: {start} raw: {:?}",
            String::from_utf8_lossy(&c.raw.lock().unwrap())
        );
        assert!(
            !start.contains("Settings Error"),
            "no settings error: {start}"
        );
        c.wait(1000);
        c.pty.write(b"/status").unwrap();
        c.wait_for("/status", 5000);
        c.pty.write(b"\r").unwrap();
        let text = c.wait_for("Setting sources", 8000);
        println!("{text}");
        c.pty.write(b"\x1b").unwrap();
        c.wait(500);
    }

    /// Brief replies: starts the Claude TUI with the `focuside-brief` plugin, runs the local
    /// `/output-style` command with no argument (it lists the styles; nothing goes to the
    /// model and nothing is selected), checks that the plugin style is loaded, and closes the
    /// list with Esc. The list marks the `outputStyle` setting as current, not the forced plugin
    /// style; in Claude Code 2.1.323 the system prompt takes a plugin style with
    /// `force-for-plugin` before that setting. Needs the built helper:
    /// `cargo test -p focuside-desktop -- --ignored real_claude_brief_style --nocapture`.
    #[test]
    #[ignore]
    fn real_claude_brief_style() {
        let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
        let data = tempfile::tempdir().unwrap();
        let helper = crate::hooks::helper_path().expect("focuside-hook built");
        let plugin = crate::hooks::ensure_plugin(data.path(), &helper, true).unwrap();
        let events = data.path().join("events");
        let launch = crate::pane::launch_for(tui, &events, Some(&plugin));
        let mut c = ClaudeScreen::start(launch);
        // The status line is the last thing the TUI draws at start.
        c.wait_for("shift+tab to cycle", 30000);
        c.wait(1000);
        c.pty.write(b"/output-style").unwrap();
        let typed = c.wait_for("/output-style", 5000);
        assert!(typed.contains("/output-style"), "the TUI took the command");
        c.pty.write(b"\r").unwrap();
        let text = c.wait_for("brief replies", 5000);
        println!("{text}");
        c.pty.write(b"\x1b").unwrap();
        c.wait(500);
        assert!(
            text.contains("focuside:FocusIDE brief replies"),
            "the brief style is loaded as a plugin style"
        );
    }

    /// Claude's transcript folder for a project folder: `~/.claude/projects/<path with every
    /// character that is not a letter or digit replaced by '-'>`. Tests only: FocusIDE never
    /// reads transcripts; panes get the session id from hook events.
    fn transcripts_of(cwd: &Path) -> std::path::PathBuf {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .expect("home folder");
        let name: String = crate::dunce(cwd.canonicalize().unwrap())
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        std::path::PathBuf::from(home)
            .join(".claude/projects")
            .join(name)
    }

    fn transcript_ids(dir: &Path) -> Vec<(std::time::SystemTime, String)> {
        let mut out: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
            .filter_map(|e| {
                let t = e.metadata().ok()?.modified().ok()?;
                Some((t, e.path().file_stem()?.to_string_lossy().to_string()))
            })
            .collect();
        out.sort();
        out
    }

    /// Resume in a Claude pane, live: starts the Claude TUI in `target/resume-live` (inside
    /// this trusted repo), types one short prompt so a conversation exists (the test types it;
    /// FocusIDE never does), closes the TUI, takes the session id from the newest transcript
    /// file name (test only), then starts the pane's resume launch (`claude --resume <id>`)
    /// and checks that the TUI starts with that conversation and no error. Last, an unknown
    /// id: the TUI must exit with an error code within `RESUME_FAIL_WINDOW`, which is how a
    /// pane detects a failed resume.
    /// `cargo test -p focuside-desktop -- --ignored real_claude_resume --nocapture`.
    /// Run it from a plain terminal: inside a Claude Code session the variables of that session
    /// (`CLAUDECODE`, `CLAUDE_CODE_CHILD_SESSION`, ...) make the test's Claude a child session
    /// that writes no transcript of its own.
    #[test]
    #[ignore]
    fn real_claude_resume() {
        assert!(
            std::env::var_os("CLAUDE_CODE_CHILD_SESSION").is_none()
                && std::env::var_os("CLAUDECODE").is_none(),
            "run this test outside a Claude Code session"
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let cwd = root.join("target/resume-live");
        std::fs::create_dir_all(&cwd).unwrap();
        let transcripts = transcripts_of(&cwd);
        let before = transcript_ids(&transcripts);
        let launch = |args: Vec<String>| {
            let tui = focuside_core::agent::tui_command("claude").expect("claude installed");
            let mut all = tui.args.clone();
            all.extend(args);
            Launch {
                program: tui.program.to_string_lossy().to_string(),
                args: all,
                env: vec![],
            }
        };

        // 1. A fresh TUI and one short prompt.
        let mut c = ClaudeScreen::start_in(launch(vec![]), &cwd);
        let screen = c.wait_for("shift+tab to cycle", 40000);
        assert!(
            screen.contains("shift+tab to cycle"),
            "the TUI started: {screen} raw: {:?}",
            String::from_utf8_lossy(&c.raw.lock().unwrap())
        );
        c.wait(3000);
        let prompt = "Reply with the single word: resumed";
        // The TUI can drop the first keys right after it starts: type again until it shows.
        let mut typed = String::new();
        for _ in 0..5 {
            c.pty.write(prompt.as_bytes()).unwrap();
            typed = c.wait_for("single word", 6000);
            if typed.contains("single word") {
                break;
            }
        }
        assert!(
            typed.contains("single word"),
            "the TUI took the text: {typed}"
        );
        c.wait(500);
        c.pty.write(b"\r").unwrap();
        let mut id = None;
        for i in 0..450 {
            let screen = c.wait(200);
            if i == 50 {
                println!("after Enter:\n{screen}");
            }
            let new: Vec<_> = transcript_ids(&transcripts)
                .into_iter()
                .filter(|x| !before.iter().any(|b| b.1 == x.1))
                .collect();
            if let Some((_, n)) = new.last() {
                let text = std::fs::read_to_string(transcripts.join(format!("{n}.jsonl")))
                    .unwrap_or_default();
                if text.contains("\"type\":\"assistant\"") {
                    id = Some(n.clone());
                    break;
                }
            }
        }
        let id = id.unwrap_or_else(|| {
            panic!(
                "a new transcript with an answer in {}; screen:\n{}",
                transcripts.display(),
                c.wait(100)
            )
        });
        println!("session id: {id}");
        c.wait(2000);
        drop(c);

        // 2. The pane's resume launch brings that conversation back.
        let mut tui = focuside_core::agent::tui_command("claude").unwrap();
        assert!(tui.resume("claude", &id));
        let resume_args: Vec<String> = tui.args.clone();
        assert!(resume_args.ends_with(&["--resume".to_string(), id.clone()]));
        let mut r = ClaudeScreen::start_in(
            Launch {
                program: tui.program.to_string_lossy().to_string(),
                args: resume_args,
                env: vec![],
            },
            &cwd,
        );
        let screen = r.wait_for("shift+tab to cycle", 40000);
        println!("resumed screen:\n{screen}");
        assert!(
            screen.contains("shift+tab to cycle"),
            "the resumed TUI started"
        );
        assert!(
            !screen.to_lowercase().contains("no conversation found"),
            "no resume error"
        );
        assert!(
            screen.contains("Reply with the single word"),
            "the earlier prompt is on the screen"
        );
        r.wait(1500);
        assert_eq!(r.exited(), None, "the resumed TUI still runs");
        drop(r);

        // 3. An unknown id: the TUI exits with an error soon.
        let mut bad = ClaudeScreen::start_in(
            launch(vec![
                "--resume".into(),
                "00000000-0000-4000-8000-000000000000".into(),
            ]),
            &cwd,
        );
        let started = std::time::Instant::now();
        while bad.exited().is_none() && started.elapsed() < Duration::from_secs(30) {
            bad.wait(200);
        }
        let took = started.elapsed();
        println!(
            "unknown id: exit {:?} after {took:?}\n{}",
            bad.exited(),
            bad.wait(100)
        );
        let code = bad.exited().expect("the TUI exited");
        assert_ne!(code, Some(0), "an error exit code");
        assert!(
            took < crate::pane::RESUME_FAIL_WINDOW,
            "within the resume fail window"
        );
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
            move |_| {
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
