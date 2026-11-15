//! The project's own git repository, for the Git panel: status, stage, commit, branches,
//! history and remote commands. Runs the system `git` binary with `std::process::Command`.
//!
//! This module never touches the shadow repo of `snapshot.rs`: every command runs in the
//! project folder with `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE` removed from the
//! environment, so git finds the user's repository the normal way.
//!
//! Remote commands (fetch, pull, push) use the user's own git setup, including the credential
//! helper. FocusIDE never reads, stores or asks for credentials. `GIT_TERMINAL_PROMPT=0` keeps
//! git from waiting for a password on a terminal nobody sees.
//!
//! Paths are relative to the repository root with forward slashes, as git prints them. The
//! project folder can be a folder inside the repository; `Status::prefix` is its path there.

use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// At most this many entries in a status. Huge untracked folders stay usable.
pub const MAX_ENTRIES: usize = 5000;
/// Files larger than this show as binary in a diff.
pub const MAX_DIFF_BYTES: usize = 4 * 1024 * 1024;
/// Fetch, pull and push stop after this long (for example when ssh waits for a passphrase).
pub const REMOTE_TIMEOUT: Duration = Duration::from_secs(180);
/// Local commands stop after this long.
pub const LOCAL_TIMEOUT: Duration = Duration::from_secs(60);

/// One changed path from `git status --porcelain=v2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    /// The path before a rename or copy in the index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig: Option<String>,
    /// Index status letter: M, A, D, R, C, T, U, or '.' for no change.
    pub x: char,
    /// Work tree status letter, same letters.
    pub y: char,
    /// "change", "rename", "conflict" or "untracked".
    pub kind: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// False when the project folder is not in a git repository. Everything else is empty then.
    pub repo: bool,
    /// The repository root (git's `--show-toplevel`).
    pub root: String,
    /// The project folder inside the repository, with a trailing slash, or "" at the root.
    pub prefix: String,
    /// The branch name. None when HEAD is detached.
    pub branch: Option<String>,
    /// The commit HEAD points to (short), or None before the first commit.
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<Entry>,
    /// More than `MAX_ENTRIES` entries; the rest are left out.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    /// Short name: "main" or "origin/main".
    pub name: String,
    pub remote: bool,
    pub current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    /// Status letter: M, A, D, R, C, T.
    pub status: char,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig: Option<String>,
}

/// Two sides of a file for the diff view. None: the file does not exist on that side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub binary: bool,
}

/// Which change a diff shows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DiffSource {
    /// Work tree against the index.
    Unstaged,
    /// Index against HEAD.
    Staged,
    /// A commit against its first parent.
    Commit { hash: String },
}

/// The user's repository for one project folder.
#[derive(Debug, Clone)]
pub struct Repo {
    git: PathBuf,
    /// The project folder.
    dir: PathBuf,
    /// The repository root, found once. Commands run there, so paths are relative to the
    /// root as git's status prints them.
    top: OnceLock<Option<PathBuf>>,
}

impl Repo {
    pub fn new(git: &Path, dir: &Path) -> Self {
        Self {
            git: git.to_path_buf(),
            dir: dir.to_path_buf(),
            top: OnceLock::new(),
        }
    }

    fn command(&self) -> Command {
        let dir = self.toplevel().unwrap_or_else(|| self.dir.clone());
        self.command_in(&dir)
    }

    fn command_in(&self, dir: &Path) -> Command {
        let mut c = Command::new(&self.git);
        c.current_dir(dir)
            // Pathspecs are file names, never patterns.
            .arg("--literal-pathspecs")
            // Status never writes the index, so it does not wake the watcher.
            .arg("--no-optional-locks")
            .args(["-c", "core.quotePath=false", "-c", "color.ui=false"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_MERGE_AUTOEDIT", "no")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window for children of a GUI app.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            c.creation_flags(CREATE_NO_WINDOW);
        }
        c
    }

    /// Runs git and returns stdout. On failure the error is git's own message.
    fn run(&self, args: &[&str], paths: &[String]) -> Result<Vec<u8>, String> {
        self.run_with(args, paths, None, LOCAL_TIMEOUT)
    }

    fn run_with(
        &self,
        args: &[&str],
        paths: &[String],
        input: Option<&str>,
        timeout: Duration,
    ) -> Result<Vec<u8>, String> {
        run_command(self.command(), args, paths, input, timeout)
    }

    fn text(&self, args: &[&str]) -> Result<String, String> {
        self.run(args, &[])
            .map(|b| String::from_utf8_lossy(&b).trim_end().to_string())
    }

    /// Runs git in the project folder itself, not in the repository root.
    fn text_here(&self, args: &[&str]) -> Result<String, String> {
        run_command(self.command_in(&self.dir), args, &[], None, LOCAL_TIMEOUT)
            .map(|b| String::from_utf8_lossy(&b).trim_end().to_string())
    }

    /// The repository root, or None when the folder is not in a repository. Found once, so
    /// after `init` the caller makes a new `Repo`.
    pub fn toplevel(&self) -> Option<PathBuf> {
        self.top
            .get_or_init(|| {
                self.text_here(&["rev-parse", "--show-toplevel"])
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .clone()
    }

    /// `git init` in the project folder.
    pub fn init(&self) -> Result<(), String> {
        self.text_here(&["init", "-q"]).map(|_| ())
    }

    pub fn status(&self) -> Result<Status, String> {
        let Some(root) = self.toplevel() else {
            return Ok(Status::default());
        };
        let prefix = self
            .text_here(&["rev-parse", "--show-prefix"])
            .unwrap_or_default();
        let out = self.run(
            &[
                "status",
                "--porcelain=v2",
                "-z",
                "--branch",
                "--untracked-files=all",
                "--ignore-submodules=dirty",
            ],
            &[],
        )?;
        let mut st = parse_status(&out);
        st.repo = true;
        st.root = root.to_string_lossy().replace('\\', "/");
        st.prefix = prefix;
        Ok(st)
    }

    fn has_head(&self) -> bool {
        self.run(&["rev-parse", "--verify", "-q", "HEAD"], &[])
            .is_ok()
    }

    /// Stages files (new, changed and deleted).
    pub fn stage(&self, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        self.run(&["add", "-A"], paths).map(|_| ())
    }

    pub fn stage_all(&self) -> Result<(), String> {
        self.run(&["add", "-A"], &[]).map(|_| ())
    }

    /// Takes files out of the index, back to HEAD. Before the first commit the files leave the
    /// index and become untracked.
    pub fn unstage(&self, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        if self.has_head() {
            self.run(&["reset", "-q", "HEAD"], paths).map(|_| ())
        } else {
            self.run(&["rm", "--cached", "-r", "-q", "--ignore-unmatch"], paths)
                .map(|_| ())
        }
    }

    pub fn unstage_all(&self) -> Result<(), String> {
        if self.has_head() {
            self.run(&["reset", "-q", "HEAD"], &[]).map(|_| ())
        } else {
            self.run(
                &["rm", "--cached", "-r", "-q", "--ignore-unmatch", "."],
                &[],
            )
            .map(|_| ())
        }
    }

    /// Discards work tree changes: tracked files go back to the index version, untracked files
    /// are deleted. The callers ask the user first.
    pub fn discard(&self, tracked: &[String], untracked: &[String]) -> Result<(), String> {
        if !tracked.is_empty() {
            self.run(&["checkout", "-q"], tracked)?;
        }
        if !untracked.is_empty() {
            self.run(&["clean", "-f", "-q"], untracked)?;
        }
        Ok(())
    }

    /// Commits the index. With `all`, stages every change first (new files too).
    pub fn commit(&self, message: &str, all: bool) -> Result<String, String> {
        if message.trim().is_empty() {
            return Err("The commit message is empty.".into());
        }
        if all {
            self.stage_all()?;
        }
        self.run_with(
            &["commit", "-q", "--cleanup=strip", "-F", "-"],
            &[],
            Some(message),
            LOCAL_TIMEOUT,
        )?;
        self.text(&["rev-parse", "--short", "HEAD"])
    }

    /// Local branches first (newest commit first), then remote branches.
    pub fn branches(&self) -> Result<Vec<Branch>, String> {
        let out = self.run(
            &[
                "for-each-ref",
                "--sort=-committerdate",
                "--format=%(refname)%00%(refname:short)%00%(upstream:short)%00%(HEAD)",
                "refs/heads",
                "refs/remotes",
            ],
            &[],
        )?;
        let mut list = parse_branches(&String::from_utf8_lossy(&out));
        list.sort_by_key(|b| b.remote);
        Ok(list)
    }

    /// Switches to a local branch, or creates a local branch that tracks a remote one.
    pub fn switch(&self, name: &str, remote: bool) -> Result<(), String> {
        check_name(name)?;
        if remote {
            self.run(&["switch", "-q", "--track", name], &[])
                .map(|_| ())
        } else {
            self.run(&["switch", "-q", name], &[]).map(|_| ())
        }
    }

    /// Creates a branch at HEAD and switches to it.
    pub fn create_branch(&self, name: &str) -> Result<(), String> {
        check_name(name)?;
        self.run(&["check-ref-format", "--branch", name], &[])
            .map_err(|_| format!("'{name}' is not a valid branch name."))?;
        self.run(&["switch", "-q", "-c", name], &[]).map(|_| ())
    }

    pub fn fetch(&self) -> Result<(), String> {
        self.run_with(&["fetch", "--all", "--prune"], &[], None, REMOTE_TIMEOUT)
            .map(|_| ())
    }

    /// `git pull` with the user's own pull settings (merge or rebase).
    pub fn pull(&self) -> Result<(), String> {
        self.run_with(&["pull", "--no-edit"], &[], None, REMOTE_TIMEOUT)
            .map(|_| ())
    }

    /// `git push`. A branch without an upstream is pushed to "origin" (or the only remote) and
    /// set as upstream, like VS Code's "Publish Branch".
    pub fn push(&self, has_upstream: bool) -> Result<(), String> {
        if !has_upstream {
            let remotes = self.text(&["remote"])?;
            let list: Vec<&str> = remotes.lines().filter(|l| !l.is_empty()).collect();
            let remote = if list.contains(&"origin") {
                Some("origin")
            } else if list.len() == 1 {
                Some(list[0])
            } else {
                None
            };
            match remote {
                Some(r) if !r.starts_with('-') => {
                    return self
                        .run_with(&["push", "-u", r, "HEAD"], &[], None, REMOTE_TIMEOUT)
                        .map(|_| ());
                }
                _ if list.is_empty() => {
                    return Err(
                        "This repository has no remote. Add one with git remote add.".into(),
                    )
                }
                _ => {}
            }
        }
        self.run_with(&["push"], &[], None, REMOTE_TIMEOUT)
            .map(|_| ())
    }

    /// The newest commits of HEAD. Empty before the first commit.
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>, String> {
        if !self.has_head() {
            return Ok(Vec::new());
        }
        let n = format!("-n{limit}");
        let out = self.run(
            &[
                "log",
                &n,
                "--no-show-signature",
                "--format=%H%x00%h%x00%an%x00%at%x00%s%x1e",
            ],
            &[],
        )?;
        Ok(parse_log(&String::from_utf8_lossy(&out)))
    }

    /// The files a commit changed, against its first parent.
    pub fn commit_files(&self, hash: &str) -> Result<Vec<CommitFile>, String> {
        check_rev(hash)?;
        let parent = self.first_parent(hash)?;
        let out = match parent {
            Some(p) => self.run(&["diff", "-z", "--name-status", "-M", &p, hash], &[])?,
            None => self.run(
                &[
                    "diff-tree",
                    "-z",
                    "-r",
                    "--root",
                    "--no-commit-id",
                    "--name-status",
                    hash,
                ],
                &[],
            )?,
        };
        Ok(parse_name_status(&out))
    }

    fn first_parent(&self, hash: &str) -> Result<Option<String>, String> {
        let line = self.text(&["rev-list", "--parents", "-n1", hash])?;
        Ok(line.split_whitespace().nth(1).map(String::from))
    }

    /// A file at a revision ("HEAD", "" for the index, or a commit), as checkout would write it
    /// (line endings and filters applied). None when the file is not there.
    pub fn show(&self, rev: &str, path: &str) -> Option<Vec<u8>> {
        if !rev.is_empty() && check_rev(rev).is_err() {
            return None;
        }
        let spec = format!("{rev}:{path}");
        self.run(&["cat-file", "--filters", &spec], &[]).ok()
    }

    /// The file in the work tree. `path` is relative to the repository root.
    fn work_file(&self, path: &str) -> Option<Vec<u8>> {
        let root = self.toplevel()?;
        let full = root.join(path);
        if !full.starts_with(&root) || path.split('/').any(|p| p == "..") {
            return None;
        }
        std::fs::read(full).ok()
    }

    /// Both sides of one file of a change, for the diff view.
    pub fn versions(&self, source: &DiffSource, path: &str, orig: Option<&str>) -> Versions {
        let before_path = orig.unwrap_or(path);
        let (before, after) = match source {
            DiffSource::Unstaged => (
                self.show("", path).or_else(|| self.show("HEAD", path)),
                self.work_file(path),
            ),
            DiffSource::Staged => (self.show("HEAD", before_path), self.show("", path)),
            DiffSource::Commit { hash } => {
                let parent = self.first_parent(hash).ok().flatten();
                (
                    parent.and_then(|p| self.show(&p, before_path)),
                    self.show(hash, path),
                )
            }
        };
        to_versions(path, before, after)
    }

    /// The HEAD version of a file as text, for the change markers in the editor. None when the
    /// file is not in HEAD or is binary.
    pub fn head_text(&self, path: &str) -> Option<String> {
        let b = self.show("HEAD", path)?;
        if is_binary(&b) {
            return None;
        }
        String::from_utf8(b).ok()
    }
}

/// Runs a git command and returns stdout. On failure the error is git's own message.
fn run_command(
    mut c: Command,
    args: &[&str],
    paths: &[String],
    input: Option<&str>,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    c.args(args);
    if !paths.is_empty() {
        c.arg("--").args(paths);
    }
    if input.is_some() {
        c.stdin(Stdio::piped());
    }
    let mut child = c.spawn().map_err(|e| format!("git: {e}"))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(text.as_bytes());
    }
    // Read both pipes on threads, so a full pipe never blocks git.
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let out_t = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err_t = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "git {} did not finish in {} seconds and was stopped.",
                    args.first().copied().unwrap_or(""),
                    timeout.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(format!("git: {e}")),
        }
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = err_t.join().unwrap_or_default();
    if status.success() {
        return Ok(stdout);
    }
    let mut msg = String::from_utf8_lossy(&stderr).trim().to_string();
    if msg.is_empty() {
        msg = String::from_utf8_lossy(&stdout).trim().to_string();
    }
    if msg.is_empty() {
        msg = format!("git {} failed ({status}).", args.join(" "));
    }
    Err(msg)
}

fn to_versions(path: &str, before: Option<Vec<u8>>, after: Option<Vec<u8>>) -> Versions {
    let binary = [&before, &after]
        .iter()
        .any(|b| b.as_ref().is_some_and(|b| is_binary(b)));
    let text = |b: Option<Vec<u8>>| b.map(|b| String::from_utf8_lossy(&b).into_owned());
    if binary {
        return Versions {
            path: path.to_string(),
            before: before.map(|_| String::new()),
            after: after.map(|_| String::new()),
            binary: true,
        };
    }
    Versions {
        path: path.to_string(),
        before: text(before),
        after: text(after),
        binary: false,
    }
}

fn is_binary(b: &[u8]) -> bool {
    b.len() > MAX_DIFF_BYTES || b[..b.len().min(8000)].contains(&0)
}

/// Branch names never start with "-", so they can never look like an option.
fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') || name.contains(char::is_whitespace) {
        return Err(format!("'{name}' is not a valid branch name."));
    }
    Ok(())
}

/// Commit ids and revisions from the UI: letters, digits and a few safe signs, never a flag.
fn check_rev(rev: &str) -> Result<(), String> {
    let ok = !rev.is_empty()
        && !rev.starts_with('-')
        && rev
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '^' | '~' | '/' | '_' | '.'));
    if ok {
        Ok(())
    } else {
        Err(format!("Not a commit: {rev}"))
    }
}

/// Parses `git status --porcelain=v2 -z --branch`.
pub fn parse_status(out: &[u8]) -> Status {
    let mut st = Status::default();
    let text = String::from_utf8_lossy(out);
    let mut fields = text.split('\0');
    while let Some(rec) = fields.next() {
        if rec.is_empty() {
            continue;
        }
        if let Some(h) = rec.strip_prefix("# ") {
            let (key, value) = h.split_once(' ').unwrap_or((h, ""));
            match key {
                "branch.oid" if value != "(initial)" => {
                    st.head = Some(value.chars().take(7).collect());
                }
                "branch.head" if value != "(detached)" => st.branch = Some(value.to_string()),
                "branch.upstream" => st.upstream = Some(value.to_string()),
                "branch.ab" => {
                    for part in value.split(' ') {
                        if let Some(n) = part.strip_prefix('+') {
                            st.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            st.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        let entry = match rec.as_bytes()[0] {
            // 1 XY sub mH mI mW hH hI path
            b'1' => {
                let p: Vec<&str> = rec.splitn(9, ' ').collect();
                (p.len() == 9).then(|| change(p[1], p[8], None, "change"))
            }
            // 2 XY sub mH mI mW hH hI Xscore path NUL origPath
            b'2' => {
                let p: Vec<&str> = rec.splitn(10, ' ').collect();
                let orig = fields.next().map(String::from);
                (p.len() == 10).then(|| change(p[1], p[9], orig, "rename"))
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 path
            b'u' => {
                let p: Vec<&str> = rec.splitn(11, ' ').collect();
                (p.len() == 11).then(|| change(p[1], p[10], None, "conflict"))
            }
            b'?' => rec.get(2..).map(|path| Entry {
                path: path.to_string(),
                orig: None,
                x: '?',
                y: '?',
                kind: "untracked",
            }),
            _ => None,
        };
        if let Some(e) = entry {
            if st.entries.len() >= MAX_ENTRIES {
                st.truncated = true;
            } else {
                st.entries.push(e);
            }
        }
    }
    st
}

fn change(xy: &str, path: &str, orig: Option<String>, kind: &'static str) -> Entry {
    let mut c = xy.chars();
    Entry {
        path: path.to_string(),
        orig,
        x: c.next().unwrap_or('.'),
        y: c.next().unwrap_or('.'),
        kind,
    }
}

fn parse_branches(text: &str) -> Vec<Branch> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\0').collect();
        if f.len() < 4 {
            continue;
        }
        let remote = f[0].starts_with("refs/remotes/");
        // "origin/HEAD" is a pointer, not a branch, and "origin" alone is its short form.
        if remote && (f[0].ends_with("/HEAD") || !f[1].contains('/')) {
            continue;
        }
        out.push(Branch {
            name: f[1].to_string(),
            remote,
            current: f[3] == "*",
            upstream: (!f[2].is_empty()).then(|| f[2].to_string()),
        });
    }
    out
}

fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\x1e')
        .filter_map(|rec| {
            let rec = rec.trim_start_matches(['\n', '\r']);
            let f: Vec<&str> = rec.splitn(5, '\0').collect();
            (f.len() == 5).then(|| Commit {
                hash: f[0].to_string(),
                short: f[1].to_string(),
                author: f[2].to_string(),
                time: f[3].parse().unwrap_or(0),
                subject: f[4].trim_end().to_string(),
            })
        })
        .collect()
}

/// Parses `--name-status -z`: "M\0path\0", "R100\0old\0new\0".
fn parse_name_status(out: &[u8]) -> Vec<CommitFile> {
    let text = String::from_utf8_lossy(out);
    let mut f = text.split('\0').filter(|s| !s.is_empty());
    let mut list = Vec::new();
    while let Some(code) = f.next() {
        let status = code.chars().next().unwrap_or('M');
        if status == 'R' || status == 'C' {
            let (Some(old), Some(new)) = (f.next(), f.next()) else {
                break;
            };
            list.push(CommitFile {
                status,
                path: new.to_string(),
                orig: Some(old.to_string()),
            });
        } else if let Some(p) = f.next() {
            list.push(CommitFile {
                status,
                path: p.to_string(),
                orig: None,
            });
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git() -> PathBuf {
        crate::which::find("git").expect("git on PATH")
    }

    /// A new repository with a user name, so commits work on any machine.
    fn repo() -> (tempfile::TempDir, Repo) {
        let d = tempfile::tempdir().unwrap();
        let r = Repo::new(&git(), d.path());
        r.init().unwrap();
        r.run(&["config", "user.name", "Test"], &[]).unwrap();
        r.run(&["config", "user.email", "test@example.com"], &[])
            .unwrap();
        r.run(&["config", "core.autocrlf", "false"], &[]).unwrap();
        r.run(&["config", "commit.gpgsign", "false"], &[]).unwrap();
        (d, r)
    }

    fn write(d: &Path, path: &str, text: &str) {
        let p = d.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn find<'a>(st: &'a Status, path: &str) -> &'a Entry {
        st.entries
            .iter()
            .find(|e| e.path == path)
            .unwrap_or_else(|| panic!("{path} not in {:?}", st.entries))
    }

    #[test]
    fn parses_porcelain_v2() {
        let out = b"# branch.oid 1234567890abcdef\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -3\0\
1 .M N... 100644 100644 100644 aaaa bbbb src/a b.ts\0\
2 R. N... 100644 100644 100644 aaaa bbbb R100 new name.ts\0old.ts\0\
u UU N... 100644 100644 100644 100644 aaaa bbbb cccc both.ts\0\
? dir/new.txt\0! ignored.log\0";
        let st = parse_status(out);
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert_eq!(st.head.as_deref(), Some("1234567"));
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!((st.ahead, st.behind), (2, 3));
        assert_eq!(st.entries.len(), 4);
        assert_eq!(st.entries[0].path, "src/a b.ts");
        assert_eq!((st.entries[0].x, st.entries[0].y), ('.', 'M'));
        assert_eq!(st.entries[1].path, "new name.ts");
        assert_eq!(st.entries[1].orig.as_deref(), Some("old.ts"));
        assert_eq!(st.entries[1].kind, "rename");
        assert_eq!(st.entries[2].kind, "conflict");
        assert_eq!(st.entries[2].path, "both.ts");
        assert_eq!(st.entries[3].kind, "untracked");
        assert_eq!(st.entries[3].path, "dir/new.txt");
    }

    #[test]
    fn parses_initial_and_detached() {
        let st = parse_status(b"# branch.oid (initial)\0# branch.head main\0");
        assert_eq!(st.head, None);
        assert_eq!(st.branch.as_deref(), Some("main"));
        let st = parse_status(b"# branch.oid abcdef1234\0# branch.head (detached)\0");
        assert_eq!(st.branch, None);
        assert_eq!(st.head.as_deref(), Some("abcdef1"));
    }

    #[test]
    fn parses_log_and_name_status() {
        let log = parse_log(
            "aaa\0a\0Ann\x001700000000\0First line\x1e\nbbb\0b\0Bo\x001700000001\0Second\x1e\n",
        );
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].subject, "First line");
        assert_eq!(log[1].time, 1700000001);
        let files = parse_name_status(b"M\0a.txt\0R100\0old.txt\0new.txt\0D\0gone.txt\0");
        assert_eq!(files.len(), 3);
        assert_eq!(files[1].orig.as_deref(), Some("old.txt"));
        assert_eq!(files[1].path, "new.txt");
        assert_eq!(files[2].status, 'D');
    }

    #[test]
    fn not_a_repo() {
        let d = tempfile::tempdir().unwrap();
        // A temp folder can sit inside a repository on a developer machine; only check when not.
        let r = Repo::new(&git(), d.path());
        if r.toplevel().is_none() {
            let st = r.status().unwrap();
            assert!(!st.repo);
            assert!(st.entries.is_empty());
        }
    }

    #[test]
    fn status_stage_unstage_commit() {
        let (d, r) = repo();
        write(d.path(), "a.txt", "one\n");
        write(d.path(), "sub dir/b.txt", "two\n");
        let st = r.status().unwrap();
        assert!(st.repo);
        assert_eq!(st.head, None, "no commit yet");
        assert_eq!(find(&st, "a.txt").kind, "untracked");
        assert_eq!(find(&st, "sub dir/b.txt").kind, "untracked");

        // Stage one file before the first commit, then take it out again.
        r.stage(&["a.txt".into()]).unwrap();
        assert_eq!(find(&r.status().unwrap(), "a.txt").x, 'A');
        r.unstage(&["a.txt".into()]).unwrap();
        assert_eq!(find(&r.status().unwrap(), "a.txt").kind, "untracked");

        // Commit everything when nothing is staged.
        let short = r.commit("First\n\nBody", true).unwrap();
        assert!(!short.is_empty());
        let st = r.status().unwrap();
        assert!(st.entries.is_empty(), "{:?}", st.entries);
        assert!(st.head.is_some());
        let log = r.log(50).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].subject, "First");
        assert_eq!(log[0].author, "Test");

        // Change, stage, unstage with a HEAD.
        write(d.path(), "a.txt", "one\nmore\n");
        assert_eq!(
            (
                find(&r.status().unwrap(), "a.txt").x,
                find(&r.status().unwrap(), "a.txt").y
            ),
            ('.', 'M')
        );
        r.stage(&["a.txt".into()]).unwrap();
        let e = r.status().unwrap();
        assert_eq!((find(&e, "a.txt").x, find(&e, "a.txt").y), ('M', '.'));
        let v = r.versions(&DiffSource::Staged, "a.txt", None);
        assert_eq!(v.before.as_deref(), Some("one\n"));
        assert_eq!(v.after.as_deref(), Some("one\nmore\n"));
        r.unstage(&["a.txt".into()]).unwrap();
        let v = r.versions(&DiffSource::Unstaged, "a.txt", None);
        assert_eq!(v.before.as_deref(), Some("one\n"));
        assert_eq!(v.after.as_deref(), Some("one\nmore\n"));
        assert_eq!(r.head_text("a.txt").as_deref(), Some("one\n"));
        assert_eq!(r.head_text("missing.txt"), None);

        // A deleted file stages as a deletion.
        std::fs::remove_file(d.path().join("sub dir/b.txt")).unwrap();
        assert_eq!(find(&r.status().unwrap(), "sub dir/b.txt").y, 'D');
        r.stage(&["sub dir/b.txt".into()]).unwrap();
        assert_eq!(find(&r.status().unwrap(), "sub dir/b.txt").x, 'D');

        // Commit only what is staged: a.txt stays changed.
        let second = r.commit("Remove b", false).unwrap();
        let st = r.status().unwrap();
        assert_eq!(st.entries.len(), 1);
        assert_eq!(find(&st, "a.txt").y, 'M');
        let files = r.commit_files(&second).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            (files[0].status, files[0].path.as_str()),
            ('D', "sub dir/b.txt")
        );
        let first = &r.log(50).unwrap()[1].hash;
        let files = r.commit_files(first).unwrap();
        assert_eq!(files.len(), 2, "root commit lists its files");
        let v = r.versions(
            &DiffSource::Commit {
                hash: second.clone(),
            },
            "sub dir/b.txt",
            None,
        );
        assert_eq!(v.before.as_deref(), Some("two\n"));
        assert_eq!(v.after, None);

        // Discard: the tracked file goes back, the untracked file goes away.
        write(d.path(), "new.txt", "x");
        r.discard(&["a.txt".into()], &["new.txt".into()]).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.txt")).unwrap(),
            "one\n"
        );
        assert!(!d.path().join("new.txt").exists());
        assert!(r.status().unwrap().entries.is_empty());

        assert!(r.commit("  \n", false).is_err(), "empty message");
    }

    #[test]
    fn literal_paths_and_unicode() {
        let (d, r) = repo();
        write(d.path(), "[a].txt", "a");
        write(d.path(), "a.txt", "b");
        write(d.path(), "ü ñ.txt", "c");
        r.stage(&["[a].txt".into()]).unwrap();
        let st = r.status().unwrap();
        assert_eq!(find(&st, "[a].txt").x, 'A');
        assert_eq!(find(&st, "a.txt").kind, "untracked", "no glob");
        assert_eq!(find(&st, "ü ñ.txt").kind, "untracked", "quotePath off");
    }

    #[test]
    fn branches_switch_and_create() {
        let (d, r) = repo();
        write(d.path(), "a.txt", "one\n");
        r.commit("First", true).unwrap();
        let main = r.status().unwrap().branch.unwrap();
        r.create_branch("feature/x").unwrap();
        assert_eq!(r.status().unwrap().branch.as_deref(), Some("feature/x"));
        let list = r.branches().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|b| b.name == "feature/x" && b.current));
        assert!(list
            .iter()
            .any(|b| b.name == main && !b.current && !b.remote));
        r.switch(&main, false).unwrap();
        assert_eq!(r.status().unwrap().branch.as_deref(), Some(main.as_str()));
        assert!(r.create_branch("bad name").is_err());
        assert!(r.create_branch("-x").is_err());
        assert!(r.create_branch("a..b").is_err());
        assert!(r.switch("--orphan", false).is_err());
        assert!(r.commit_files("--all").is_err());
        // Push without a remote says so plainly.
        let err = r.push(false).unwrap_err();
        assert!(err.contains("no remote"), "{err}");
    }

    #[test]
    fn ahead_of_a_local_remote_and_push() {
        let (d, r) = repo();
        let bare = tempfile::tempdir().unwrap();
        let ok = Command::new(git())
            .args(["init", "-q", "--bare"])
            .arg(bare.path())
            .status()
            .unwrap()
            .success();
        assert!(ok);
        r.run(
            &["remote", "add", "origin", &bare.path().to_string_lossy()],
            &[],
        )
        .unwrap();
        write(d.path(), "a.txt", "one\n");
        r.commit("First", true).unwrap();
        // No upstream yet: push sets it.
        r.push(false).unwrap();
        let st = r.status().unwrap();
        assert!(st.upstream.is_some(), "{st:?}");
        write(d.path(), "a.txt", "two\n");
        r.commit("Second", true).unwrap();
        let st = r.status().unwrap();
        assert_eq!((st.ahead, st.behind), (1, 0));
        r.push(true).unwrap();
        r.fetch().unwrap();
        assert_eq!(r.status().unwrap().ahead, 0);
        r.pull().unwrap();
        assert!(r.branches().unwrap().iter().any(|b| b.remote));
    }

    #[test]
    fn project_in_a_subfolder() {
        let (d, _) = repo();
        write(d.path(), "app/x.txt", "x");
        write(d.path(), "top.txt", "t");
        let r = Repo::new(&git(), &d.path().join("app"));
        let st = r.status().unwrap();
        assert_eq!(st.prefix, "app/");
        assert!(st.entries.iter().any(|e| e.path == "app/x.txt"));
        assert!(st.entries.iter().any(|e| e.path == "top.txt"));
        // Paths are relative to the repository root, also for a project in a subfolder.
        r.stage(&["app/x.txt".into()]).unwrap();
        r.stage(&["top.txt".into()]).unwrap();
        let st = r.status().unwrap();
        assert_eq!(find(&st, "app/x.txt").x, 'A');
        assert_eq!(find(&st, "top.txt").x, 'A');
    }
}
