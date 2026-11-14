//! Settings > "Plan folder": where Claude panes write their plan files (`plansDirectory`, set
//! for each pane FocusIDE starts with `--settings`, see `agent::plans_dir_args`).
//!
//! A folder inside the project is skipped by snapshots (`ShadowRepo::set_skip_dirs`) and by the
//! turn watch log, so plan files never show up as turn changes and Undo never touches them.
//! FocusIDE checks whether git ignores the folder and offers "Add to .gitignore". It edits
//! `.gitignore` only when the user clicks that button.

use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

/// Whether the project's own git ignores `folder` (relative, forward slashes). None when the
/// project is not a git repo or git fails. Runs `git check-ignore -q`: exit 0 is ignored, 1 is
/// not ignored.
pub async fn ignored(git: &Path, root: &Path, folder: &str) -> Option<bool> {
    if !root.join(".git").exists() {
        return None;
    }
    let mut cmd = Command::new(git);
    cmd.arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--"])
        .arg(format!("{folder}/"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    focuside_core::proc::prepare(&mut cmd);
    match cmd.status().await.ok()?.code()? {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}

/// The `.gitignore` line for a plan folder: anchored at the project root.
pub fn gitignore_line(folder: &str) -> String {
    format!("/{folder}/")
}

/// Adds the folder's line to `<root>/.gitignore` (creates the file when missing). Does nothing
/// when the line is already there. Only called on the user's click.
pub fn add_to_gitignore(root: &Path, folder: &str) -> Result<()> {
    let path = root.join(".gitignore");
    let line = gitignore_line(folder);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line);
    text.push('\n');
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitignore_gets_one_line() {
        let d = tempfile::tempdir().unwrap();
        add_to_gitignore(d.path(), "plans").unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join(".gitignore")).unwrap(),
            "/plans/\n"
        );
        std::fs::write(d.path().join(".gitignore"), "node_modules/").unwrap();
        add_to_gitignore(d.path(), "docs/plans").unwrap();
        add_to_gitignore(d.path(), "docs/plans").unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join(".gitignore")).unwrap(),
            "node_modules/\n/docs/plans/\n"
        );
    }

    #[tokio::test]
    async fn check_ignore_reads_the_project_rules() {
        let git = focuside_core::which::find("git").expect("git on PATH");
        let d = tempfile::tempdir().unwrap();
        assert_eq!(ignored(&git, d.path(), "plans").await, None, "not a repo");
        let ok = std::process::Command::new(&git)
            .args(["init", "-q"])
            .current_dir(d.path())
            .status()
            .unwrap()
            .success();
        assert!(ok);
        assert_eq!(ignored(&git, d.path(), "plans").await, Some(false));
        add_to_gitignore(d.path(), "plans").unwrap();
        assert_eq!(ignored(&git, d.path(), "plans").await, Some(true));
    }
}
