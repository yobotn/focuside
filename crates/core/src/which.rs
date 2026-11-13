//! Finding programs on PATH the way a shell would.
//!
//! - Windows: tries each PATHEXT extension. npm installs `.cmd` shims, which cannot be started
//!   without a shell, so a shim is followed to the program it runs.
//! - macOS: GUI apps do not get the login shell's PATH. `login_shell_path` reads it once.

use std::path::{Path, PathBuf};

/// Full path of `name` on PATH, or None.
pub fn find(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if let Some(p) = find_in(&dir, name) {
            return Some(p);
        }
    }
    None
}

fn find_in(dir: &Path, name: &str) -> Option<PathBuf> {
    if cfg!(windows) {
        let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        if Path::new(name).extension().is_some() {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
        for ext in exts.split(';').filter(|e| !e.is_empty()) {
            let p = dir.join(format!("{name}{}", ext.to_lowercase()));
            if p.is_file() {
                return Some(p);
            }
        }
        None
    } else {
        let p = dir.join(name);
        if is_executable(&p) {
            Some(p)
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// For a Windows `.cmd` or `.bat` shim, the program it starts: an `.exe` or a Node script
/// (`.js`, `.mjs`, `.cjs`, or a file without extension that starts with a node shebang) named in
/// the shim. Other paths are returned unchanged.
pub fn resolve_shim(path: &Path) -> PathBuf {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "cmd" && ext != "bat" {
        return path.to_path_buf();
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return path.to_path_buf();
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    // npm shims reference the target as "%dp0%\node_modules\...\cli.js" or "%~dp0\...".
    for token in text.split('"') {
        let t = token
            .replace("%dp0%", "")
            .replace("%~dp0", "")
            .trim_start_matches(['\\', '/'])
            .to_string();
        let lower = t.to_lowercase();
        if t.is_empty() || t.contains('%') || lower.ends_with("node.exe") {
            continue;
        }
        let p = if Path::new(&t).is_absolute() {
            PathBuf::from(&t)
        } else {
            dir.join(&t)
        };
        if (lower.ends_with(".exe") || is_node_script(&p)) && p.is_file() {
            return p;
        }
    }
    path.to_path_buf()
}

/// A file Node runs: `.js`, `.mjs` or `.cjs`, or on Windows a file without extension whose first
/// line is a node shebang (npm packages such as `cline` ship their bin that way).
pub fn is_node_script(path: &Path) -> bool {
    match path.extension().map(|e| e.to_string_lossy().to_lowercase()) {
        Some(e) => matches!(e.as_str(), "js" | "mjs" | "cjs"),
        None if cfg!(windows) => {
            use std::io::Read;
            let mut head = [0u8; 64];
            let n = std::fs::File::open(path)
                .and_then(|mut f| f.read(&mut head))
                .unwrap_or(0);
            let line = String::from_utf8_lossy(&head[..n]);
            let line = line.lines().next().unwrap_or_default();
            line.starts_with("#!") && line.contains("node")
        }
        None => false,
    }
}

/// PATH as the user's login shell sets it. Only needed on macOS, where apps started from the
/// Dock or Finder get a minimal PATH.
pub fn login_shell_path() -> Option<String> {
    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let out = std::process::Command::new(shell)
            .args(["-l", "-c", "printf '%s' \"$PATH\""])
            .output()
            .ok()?;
        let p = String::from_utf8(out.stdout).ok()?;
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// On macOS, replace this process's PATH with the login shell's, so `claude` and `git` are found
/// the same way as in a terminal. Call once at startup.
pub fn adopt_login_path() {
    if cfg!(target_os = "macos") {
        if let Some(p) = login_shell_path() {
            std::env::set_var("PATH", p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_git() {
        assert!(find("git").is_some());
    }

    /// npm shims and their backslash paths only exist on Windows.
    #[cfg(windows)]
    #[test]
    fn follows_npm_shim() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("node_modules/pkg/cli.js");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "").unwrap();
        let shim = dir.path().join("tool.cmd");
        std::fs::write(
            &shim,
            "@ECHO off\r\n\"%_prog%\"  \"%dp0%\\node_modules\\pkg\\cli.js\" %*\r\n",
        )
        .unwrap();
        let got = resolve_shim(&shim);
        assert_eq!(
            got.canonicalize().unwrap(),
            target.canonicalize().unwrap(),
            "{got:?}"
        );
    }

    /// Shims for bins that are `.mjs` files or extensionless node scripts (Kimi Code, Cline).
    #[cfg(windows)]
    #[test]
    fn follows_shim_to_mjs_and_shebang_script() {
        let dir = tempfile::tempdir().unwrap();
        for (rel, body) in [
            ("node_modules/a/dist/main.mjs", ""),
            (
                "node_modules/b/bin/b",
                "#!/usr/bin/env node\nconsole.log(1)\n",
            ),
        ] {
            let target = dir.path().join(rel);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, body).unwrap();
            let shim = dir.path().join("tool.cmd");
            let win = rel.replace('/', "\\");
            let text = format!("@ECHO off\r\n\"%_prog%\"  \"%dp0%\\{win}\" %*\r\n");
            std::fs::write(&shim, text).unwrap();
            let got = resolve_shim(&shim);
            assert_eq!(
                got.canonicalize().unwrap(),
                target.canonicalize().unwrap(),
                "{got:?}"
            );
            assert!(is_node_script(&got));
        }
        let plain = dir.path().join("plain");
        std::fs::write(&plain, "MZ").unwrap();
        assert!(!is_node_script(&plain));
    }
}
