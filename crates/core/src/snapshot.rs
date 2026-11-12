//! Snapshots, changes and undo through a shadow git repo per project.
//!
//! The shadow repo lives in app data (`--git-dir=<data>/shadow.git --work-tree=<project>`). It
//! never touches the user's own repo and works in folders that are not git repos. Every call
//! for one project goes through one queue (`ShadowRepo::lock`), so there are no `index.lock`
//! clashes.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;

/// Files over this size are not snapshotted.
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
/// Keep at most this many turns.
pub const KEEP_TURNS: usize = 50;
/// Drop turns older than this.
pub const KEEP_DAYS: u64 = 14;
/// Prune oldest turns when the shadow repo grows past this.
pub const BUDGET_BYTES: u64 = 1024 * 1024 * 1024;

/// Folders that build tools and package managers fill. Changes inside collapse to one line.
pub const GENERATED: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "out",
    "__pycache__",
    ".venv",
    "venv",
    ".gradle",
    ".turbo",
    ".cache",
    "coverage",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Uncovered {
    /// A folder with its own `.git` (separate repo or submodule).
    NestedRepo,
    BigFile,
    Symlink,
    /// Locked or read-only: git could not read it.
    Unreadable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotCovered {
    pub path: String,
    pub why: Uncovered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub commit: String,
    pub not_covered: Vec<NotCovered>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Create,
    Edit,
    Delete,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub status: Status,
    pub added: u32,
    pub removed: u32,
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    pub folder: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    pub files: Vec<FileChange>,
    pub generated: Vec<Generated>,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored: Vec<String>,
    pub failed: Vec<(String, String)>,
    /// True when a fresh snapshot shows every restored path exactly as in the source.
    pub verified: bool,
}

pub struct ShadowRepo {
    git_dir: PathBuf,
    work_tree: PathBuf,
    gitconfig: PathBuf,
    git: PathBuf,
    pub lock: Mutex<()>,
}

pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub ok: bool,
}

impl ShadowRepo {
    /// Opens or creates the shadow repo for `work_tree` in `data_dir` (the project's app data
    /// folder). `git` is the git executable.
    pub async fn open(data_dir: &Path, work_tree: &Path, git: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let gitconfig = data_dir.join("gitconfig");
        if !gitconfig.exists() {
            std::fs::write(&gitconfig, "")?;
        }
        let repo = Self {
            git_dir: data_dir.join("shadow.git"),
            work_tree: work_tree.to_path_buf(),
            gitconfig,
            git: git.to_path_buf(),
            lock: Mutex::new(()),
        };
        if !repo.git_dir.join("HEAD").exists() {
            repo.init().await?;
        }
        Ok(repo)
    }

    pub fn work_tree(&self) -> &Path {
        &self.work_tree
    }

    async fn init(&self) -> Result<()> {
        let out = self.raw(&["init", "-q"], None).await?;
        if !out.ok {
            bail!("git init failed: {}", out.stderr);
        }
        let mut settings = vec![
            ("core.autocrlf", "false"),
            ("core.filemode", "false"),
            ("core.longpaths", "true"),
            ("core.quotepath", "false"),
            ("core.fsmonitor", "false"),
            ("core.untrackedCache", "true"),
            ("core.precomposeunicode", "true"),
            ("gc.auto", "0"),
            ("advice.addEmbeddedRepo", "false"),
            ("advice.detachedHead", "false"),
            ("commit.gpgsign", "false"),
        ];
        if cfg!(windows) {
            settings.push(("core.symlinks", "false"));
        }
        for (k, v) in settings {
            self.git(&["config", k, v]).await?;
        }
        let info = self.git_dir.join("info");
        std::fs::create_dir_all(&info)?;
        // Project attributes (LFS, eol, filters) must never run on snapshots.
        std::fs::write(
            info.join("attributes"),
            "* -text -filter -ident -working-tree-encoding\n",
        )?;
        std::fs::write(info.join("exclude"), "")?;
        Ok(())
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(&self.git);
        c.arg(format!("--git-dir={}", self.git_dir.display()))
            .arg(format!("--work-tree={}", self.work_tree.display()))
            .arg("--literal-pathspecs")
            .arg("-c")
            .arg(format!(
                "core.hooksPath={}",
                self.git_dir.join("no-hooks").display()
            ))
            .args(args)
            .current_dir(&self.work_tree)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &self.gitconfig)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", "FocusIDE")
            .env("GIT_AUTHOR_EMAIL", "snapshots@focuside.dev")
            .env("GIT_COMMITTER_NAME", "FocusIDE")
            .env("GIT_COMMITTER_EMAIL", "snapshots@focuside.dev")
            .env("LC_ALL", "C")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_OBJECT_DIRECTORY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::proc::prepare(&mut c);
        c
    }

    /// Runs git and returns its output whatever the exit code.
    pub async fn raw(&self, args: &[&str], stdin: Option<&[u8]>) -> Result<Output> {
        let mut child = self
            .command(args)
            .spawn()
            .with_context(|| format!("run git {}", args.join(" ")))?;
        if let Some(input) = stdin {
            let mut s = child.stdin.take().unwrap();
            s.write_all(input).await?;
            drop(s);
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().await?;
        Ok(Output {
            ok: out.status.success(),
            stdout: out.stdout,
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }

    /// Runs git and fails on a non-zero exit.
    pub async fn git(&self, args: &[&str]) -> Result<String> {
        let out = self.raw(args, None).await?;
        if !out.ok {
            bail!("git {} failed: {}", args.join(" "), out.stderr.trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Takes a snapshot of the work tree and stores it under `refname` (e.g.
    /// `refs/fi/<session>/<turn>/start`). Call with `lock` held.
    pub async fn snapshot(&self, refname: &str, message: &str) -> Result<Snapshot> {
        let root = self.work_tree.clone();
        let mut not_covered = tokio::task::spawn_blocking(move || scan(&root)).await??;
        // Excluded paths are anchored and escaped so they match only themselves.
        let mut exclude = String::new();
        for nc in &not_covered {
            exclude.push('/');
            for ch in nc.path.chars() {
                if "*?[]\\!#".contains(ch) {
                    exclude.push('\\');
                }
                exclude.push(ch);
            }
            if nc.why == Uncovered::NestedRepo {
                exclude.push('/');
            }
            exclude.push('\n');
        }
        std::fs::write(self.git_dir.join("info/exclude"), exclude)?;

        // --ignore-errors: a locked file is reported and skipped instead of failing the add.
        let add = self
            .raw(&["add", "-A", "--ignore-errors", "."], None)
            .await?;
        for line in add.stderr.lines() {
            // "error: open("path"): Permission denied" / "error: unable to index file 'path'"
            if let Some(path) = quoted_path(line) {
                let path = path.replace('\\', "/");
                if !not_covered.iter().any(|n| n.path == path) {
                    not_covered.push(NotCovered {
                        path,
                        why: Uncovered::Unreadable,
                    });
                }
            }
        }
        // Drop any gitlink (nested repo) that slipped in.
        let staged = self
            .git(&["ls-files", "-s", "-z"])
            .await
            .unwrap_or_default();
        for entry in staged.split('\0') {
            if let Some(rest) = entry.strip_prefix("160000 ") {
                if let Some((_, path)) = rest.split_once('\t') {
                    let _ = self.git(&["rm", "--cached", "-q", "--", path]).await;
                    if !not_covered.iter().any(|n| n.path == path) {
                        not_covered.push(NotCovered {
                            path: path.to_string(),
                            why: Uncovered::NestedRepo,
                        });
                    }
                }
            }
        }
        let tree = self.git(&["write-tree"]).await?;
        let parent = self
            .git(&["rev-parse", "-q", "--verify", "refs/fi/head"])
            .await
            .ok();
        let mut args = vec!["commit-tree", tree.as_str(), "-m", message];
        if let Some(p) = parent.as_deref() {
            args.push("-p");
            args.push(p);
        }
        let commit = self.git(&args).await?;
        self.git(&["update-ref", refname, &commit]).await?;
        self.git(&["update-ref", "refs/fi/head", &commit]).await?;
        not_covered.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Snapshot {
            commit,
            not_covered,
        })
    }

    /// Changes from `a` to `b`, with renames detected.
    pub async fn diff(&self, a: &str, b: &str) -> Result<Changes> {
        let names = self
            .raw(&["diff", "--name-status", "-z", "-M", a, b], None)
            .await?;
        if !names.ok {
            bail!("git diff failed: {}", names.stderr.trim());
        }
        let nums = self
            .raw(&["diff", "--numstat", "-z", "-M", a, b], None)
            .await?;
        let mut stats: BTreeMap<String, (u32, u32, bool)> = BTreeMap::new();
        {
            let text = String::from_utf8_lossy(&nums.stdout);
            let mut parts = text.split('\0');
            while let Some(head) = parts.next() {
                if head.is_empty() {
                    continue;
                }
                let mut f = head.splitn(3, '\t');
                let (Some(ad), Some(rm), Some(path)) = (f.next(), f.next(), f.next()) else {
                    continue;
                };
                let path = if path.is_empty() {
                    // Rename: old and new path follow as separate fields.
                    let _old = parts.next();
                    parts.next().unwrap_or("").to_string()
                } else {
                    path.to_string()
                };
                let binary = ad == "-";
                stats.insert(
                    path,
                    (ad.parse().unwrap_or(0), rm.parse().unwrap_or(0), binary),
                );
            }
        }
        let text = String::from_utf8_lossy(&names.stdout);
        let mut parts = text.split('\0').filter(|s| !s.is_empty());
        let mut files = Vec::new();
        while let Some(code) = parts.next() {
            let kind = code.chars().next().unwrap_or('M');
            let (status, old_path, path) = match kind {
                'R' | 'C' => {
                    let old = parts.next().unwrap_or("").to_string();
                    let new = parts.next().unwrap_or("").to_string();
                    let status = if kind == 'R' {
                        Status::Rename
                    } else {
                        Status::Create
                    };
                    (status, Some(old), new)
                }
                'A' => (Status::Create, None, parts.next().unwrap_or("").to_string()),
                'D' => (Status::Delete, None, parts.next().unwrap_or("").to_string()),
                _ => (Status::Edit, None, parts.next().unwrap_or("").to_string()),
            };
            let (added, removed, binary) = stats.get(&path).copied().unwrap_or((0, 0, false));
            files.push(FileChange {
                path,
                old_path,
                status,
                added,
                removed,
                binary,
            });
        }
        Ok(collapse_generated(files))
    }

    /// Paths that differ between `a` and `b`, without rename detection (each side listed).
    pub async fn changed_paths(&self, a: &str, b: &str) -> Result<Vec<String>> {
        let out = self
            .raw(&["diff", "--name-only", "-z", "--no-renames", a, b], None)
            .await?;
        if !out.ok {
            bail!("git diff failed: {}", out.stderr.trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }

    /// Paths of `paths` that exist in `commit`.
    async fn present_in(&self, commit: &str, paths: &[String]) -> Result<BTreeSet<String>> {
        if paths.is_empty() {
            return Ok(BTreeSet::new());
        }
        let mut input = Vec::new();
        for p in paths {
            input.extend_from_slice(p.as_bytes());
            input.push(0);
        }
        // ls-tree takes paths as arguments; batch through xargs-like chunks.
        let mut present = BTreeSet::new();
        for chunk in paths.chunks(200) {
            let mut args = vec!["ls-tree", "-r", "-z", "--name-only", commit, "--"];
            args.extend(chunk.iter().map(|s| s.as_str()));
            let out = self.raw(&args, None).await?;
            for p in String::from_utf8_lossy(&out.stdout).split('\0') {
                if !p.is_empty() {
                    present.insert(p.to_string());
                }
            }
        }
        drop(input);
        Ok(present)
    }

    /// File content at `commit`, or None when the file is not in it.
    pub async fn read_file(&self, commit: &str, path: &str) -> Result<Option<Vec<u8>>> {
        let spec = format!("{commit}:{path}");
        let out = self.raw(&["cat-file", "blob", &spec], None).await?;
        Ok(if out.ok { Some(out.stdout) } else { None })
    }

    /// Makes each of `paths` in the work tree exactly as in `source`: written from the
    /// snapshot, or deleted when `source` does not have it. Call with `lock` held.
    pub async fn restore(&self, source: &str, paths: &[String]) -> Result<RestoreResult> {
        let mut result = RestoreResult::default();
        if paths.is_empty() {
            result.verified = true;
            return Ok(result);
        }
        let present = self.present_in(source, paths).await?;
        let (write, delete): (Vec<&String>, Vec<&String>) =
            paths.iter().partition(|p| present.contains(*p));

        // Write files back from the snapshot, one batch first, then one by one on failure.
        if !write.is_empty() {
            let mut input = Vec::new();
            for p in &write {
                input.extend_from_slice(p.as_bytes());
                input.push(0);
            }
            let src = format!("--source={source}");
            let out = self
                .raw(
                    &[
                        "restore",
                        &src,
                        "--worktree",
                        "--pathspec-from-file=-",
                        "--pathspec-file-nul",
                    ],
                    Some(&input),
                )
                .await?;
            if out.ok {
                result.restored.extend(write.iter().map(|p| p.to_string()));
            } else {
                for p in &write {
                    clear_readonly(&self.work_tree.join(p));
                    let one = self
                        .raw(&["restore", &src, "--worktree", "--", p], None)
                        .await?;
                    if one.ok {
                        result.restored.push(p.to_string());
                    } else {
                        result
                            .failed
                            .push((p.to_string(), one.stderr.trim().to_string()));
                    }
                }
            }
        }

        for p in delete {
            let full = self.work_tree.join(p);
            match std::fs::remove_file(&full) {
                Ok(()) => result.restored.push(p.clone()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    result.restored.push(p.clone())
                }
                Err(e) => {
                    clear_readonly(&full);
                    if std::fs::remove_file(&full).is_ok() {
                        result.restored.push(p.clone());
                    } else {
                        result.failed.push((p.clone(), e.to_string()));
                    }
                }
            }
            remove_empty_parents(&self.work_tree, &full);
        }

        // Verify against a fresh snapshot of the work tree.
        let check = self
            .snapshot("refs/fi/verify", "verify restore")
            .await
            .context("verify snapshot")?;
        let still = self.changed_paths(source, &check.commit).await?;
        let wanted: BTreeSet<&String> = result.restored.iter().collect();
        result.verified = result.failed.is_empty() && !still.iter().any(|p| wanted.contains(p));
        Ok(result)
    }

    /// Paths from `paths` that the project's ignore rules exclude, so snapshots do not cover
    /// them. Paths inside generated folders are left out.
    pub async fn ignored(&self, paths: &[String]) -> Result<Vec<String>> {
        let wanted: Vec<&String> = paths
            .iter()
            .filter(|p| !p.split('/').any(|seg| GENERATED.contains(&seg)))
            .filter(|p| !self.work_tree.join(p).is_dir())
            .collect();
        if wanted.is_empty() {
            return Ok(vec![]);
        }
        let mut input = Vec::new();
        for p in &wanted {
            input.extend_from_slice(p.as_bytes());
            input.push(0);
        }
        let out = self
            .raw(&["check-ignore", "-z", "--stdin"], Some(&input))
            .await?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }

    /// Turn refs (refs/fi/<session>/<turn>/<start|end>) with their commit time, oldest first.
    pub async fn turn_refs(&self) -> Result<Vec<(String, u64)>> {
        let out = self
            .git(&[
                "for-each-ref",
                "--format=%(refname) %(committerdate:unix)",
                "refs/fi/",
            ])
            .await?;
        let mut refs: Vec<(String, u64)> = out
            .lines()
            .filter_map(|l| {
                let (r, t) = l.rsplit_once(' ')?;
                if r == "refs/fi/head" || r == "refs/fi/verify" {
                    return None;
                }
                Some((r.to_string(), t.parse().ok()?))
            })
            .collect();
        refs.sort_by_key(|(_, t)| *t);
        Ok(refs)
    }

    /// Size of the shadow repo on disk.
    pub fn disk_size(&self) -> u64 {
        fn walk(p: &Path) -> u64 {
            let Ok(rd) = std::fs::read_dir(p) else {
                return 0;
            };
            rd.flatten()
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => walk(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        }
        walk(&self.git_dir)
    }

    /// Deletes refs of turns beyond the last `KEEP_TURNS`, older than `KEEP_DAYS`, and the
    /// oldest while over budget. Returns the removed turn prefixes (refs/fi/<session>/<turn>).
    /// Call with `lock` held. Run `gc` afterwards when the app is idle.
    pub async fn prune(&self, now_unix: u64) -> Result<Vec<String>> {
        let refs = self.turn_refs().await?;
        // Group by turn prefix, keyed by the newest ref time in the turn.
        let mut turns: BTreeMap<String, u64> = BTreeMap::new();
        for (r, t) in &refs {
            let prefix = r.rsplit_once('/').map(|(p, _)| p).unwrap_or(r).to_string();
            let e = turns.entry(prefix).or_insert(0);
            *e = (*e).max(*t);
        }
        let mut ordered: Vec<(String, u64)> = turns.into_iter().collect();
        ordered.sort_by_key(|(_, t)| *t);
        let cutoff = now_unix.saturating_sub(KEEP_DAYS * 86_400);
        let excess = ordered.len().saturating_sub(KEEP_TURNS);
        let mut drop: Vec<String> = ordered
            .iter()
            .enumerate()
            .filter(|(i, (_, t))| *i < excess || *t < cutoff)
            .map(|(_, (p, _))| p.clone())
            .collect();
        if self.disk_size() > BUDGET_BYTES {
            // Drop the oldest half of what is left; gc reclaims the space later.
            let left: Vec<&(String, u64)> =
                ordered.iter().filter(|(p, _)| !drop.contains(p)).collect();
            for (p, _) in left.iter().take(left.len() / 2) {
                drop.push(p.clone());
            }
        }
        for prefix in &drop {
            for (r, _) in refs
                .iter()
                .filter(|(r, _)| r.starts_with(&format!("{prefix}/")))
            {
                let _ = self.git(&["update-ref", "-d", r]).await;
            }
        }
        Ok(drop)
    }

    /// Removes unreachable objects. Slow on big repos; run when the app is idle.
    pub async fn gc(&self) -> Result<()> {
        self.git(&["reflog", "expire", "--expire=now", "--all"])
            .await?;
        self.git(&["gc", "-q", "--prune=now"]).await?;
        Ok(())
    }
}

/// Walks the work tree like `git add` would (respecting .gitignore) and lists what snapshots
/// cannot cover: nested repos, big files and, on Windows, symlinks.
fn scan(root: &Path) -> Result<Vec<NotCovered>> {
    let mut out = Vec::new();
    let mut walker = ignore::WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(false)
        .git_global(false)
        .require_git(false)
        .ignore(false)
        .parents(true)
        .follow_links(false);
    let root_owned = root.to_path_buf();
    walker.filter_entry(move |e| e.file_name() != ".git" || e.path() == root_owned);
    for entry in walker.build() {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path == root {
            continue;
        }
        let rel = rel_path(root, path);
        let Some(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            if path.join(".git").exists() {
                out.push(NotCovered {
                    path: rel,
                    why: Uncovered::NestedRepo,
                });
            }
            continue;
        }
        if ft.is_symlink() {
            if cfg!(windows) {
                out.push(NotCovered {
                    path: rel,
                    why: Uncovered::Symlink,
                });
            }
            continue;
        }
        if let Ok(m) = entry.metadata() {
            if m.len() > MAX_FILE_BYTES {
                out.push(NotCovered {
                    path: rel,
                    why: Uncovered::BigFile,
                });
            }
        }
    }
    // Do not list files inside nested repos.
    let repos: Vec<String> = out
        .iter()
        .filter(|n| n.why == Uncovered::NestedRepo)
        .map(|n| format!("{}/", n.path))
        .collect();
    out.retain(|n| n.why == Uncovered::NestedRepo || !repos.iter().any(|r| n.path.starts_with(r)));
    Ok(out)
}

/// Relative path with forward slashes.
pub fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn quoted_path(line: &str) -> Option<String> {
    for q in ['"', '\''] {
        if let Some(start) = line.find(q) {
            if let Some(len) = line[start + 1..].find(q) {
                return Some(line[start + 1..start + 1 + len].to_string());
            }
        }
    }
    None
}

fn clear_readonly(path: &Path) {
    if let Ok(m) = std::fs::metadata(path) {
        let mut p = m.permissions();
        if p.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            p.set_readonly(false);
            let _ = std::fs::set_permissions(path, p);
        }
    }
}

fn remove_empty_parents(root: &Path, file: &Path) {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) {
            break;
        }
        if std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

/// Moves changes inside generated folders (node_modules, target, dist...) into counts.
pub fn collapse_generated(files: Vec<FileChange>) -> Changes {
    let mut kept = Vec::new();
    let mut gen: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let folder = f
            .path
            .split('/')
            .scan(String::new(), |acc, seg| {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(seg);
                Some((acc.clone(), seg.to_string()))
            })
            .find(|(_, seg)| GENERATED.contains(&seg.as_str()))
            .map(|(prefix, _)| prefix);
        match folder {
            Some(folder) if folder != f.path => *gen.entry(format!("{folder}/")).or_default() += 1,
            _ => kept.push(f),
        }
    }
    let added = kept.iter().map(|f| f.added).sum();
    let removed = kept.iter().map(|f| f.removed).sum();
    Changes {
        files: kept,
        generated: gen
            .into_iter()
            .map(|(folder, count)| Generated { folder, count })
            .collect(),
        added,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git() -> PathBuf {
        crate::which::find("git").expect("git on PATH")
    }

    /// Every file under `root` (except .git folders) with its exact bytes.
    fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if e.file_name() == ".git" {
                    continue;
                }
                if p.is_dir() {
                    walk(root, &p, out);
                } else {
                    out.insert(rel_path(root, &p), fs::read(&p).unwrap());
                }
            }
        }
        walk(root, root, &mut out);
        out
    }

    async fn setup() -> (tempfile::TempDir, tempfile::TempDir, ShadowRepo) {
        let proj = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let p = proj.path();
        fs::create_dir_all(p.join("src")).unwrap();
        fs::write(p.join("src/a.rs"), "fn a() {}\r\n// crlf kept\r\n").unwrap();
        fs::write(p.join("src/b.rs"), "fn b() {}\n").unwrap();
        fs::write(p.join("README.md"), "# hi\n").unwrap();
        fs::write(
            p.join(".gitattributes"),
            "* text=auto eol=lf\n*.rs filter=lfs\n",
        )
        .unwrap();
        fs::write(p.join(".gitignore"), ".env\nnode_modules/\n").unwrap();
        fs::write(p.join(".env"), "SECRET=1\n").unwrap();
        fs::write(p.join("bin.dat"), [0u8, 159, 146, 150, 0, 255]).unwrap();
        let repo = ShadowRepo::open(data.path(), p, &git()).await.unwrap();
        (proj, data, repo)
    }

    #[tokio::test]
    async fn undo_restores_byte_for_byte() {
        let (proj, _data, repo) = setup().await;
        let p = proj.path();
        let before = tree(p);
        let start = repo.snapshot("refs/fi/s/1/start", "start").await.unwrap();

        // The "agent" edits, creates, deletes, renames and fills a generated folder.
        fs::write(p.join("src/a.rs"), "fn a() { changed() }\n").unwrap();
        fs::write(p.join("src/new.rs"), "fn new() {}\n").unwrap();
        fs::remove_file(p.join("README.md")).unwrap();
        fs::rename(p.join("src/b.rs"), p.join("src/c.rs")).unwrap();
        fs::create_dir_all(p.join("dist/assets/deep")).unwrap();
        fs::write(p.join("dist/assets/deep/x.js"), "x").unwrap();
        fs::write(p.join("dist/index.html"), "<html>").unwrap();
        fs::write(p.join("bin.dat"), [1u8, 2, 3]).unwrap();

        let end = repo.snapshot("refs/fi/s/1/end", "end").await.unwrap();
        let changes = repo.diff(&start.commit, &end.commit).await.unwrap();
        let by_path: BTreeMap<_, _> = changes.files.iter().map(|f| (f.path.as_str(), f)).collect();
        assert_eq!(by_path["src/a.rs"].status, Status::Edit);
        assert_eq!(by_path["src/new.rs"].status, Status::Create);
        assert_eq!(by_path["README.md"].status, Status::Delete);
        assert_eq!(by_path["src/c.rs"].status, Status::Rename);
        assert_eq!(by_path["src/c.rs"].old_path.as_deref(), Some("src/b.rs"));
        assert!(by_path["bin.dat"].binary);
        assert_eq!(
            changes.generated,
            vec![Generated {
                folder: "dist/".into(),
                count: 2
            }]
        );

        let paths = repo
            .changed_paths(&start.commit, &end.commit)
            .await
            .unwrap();
        let r = repo.restore(&start.commit, &paths).await.unwrap();
        assert!(r.failed.is_empty(), "{:?}", r.failed);
        assert!(r.verified);
        assert_eq!(tree(p), before, "folder is not byte for byte as before");
        assert!(
            !p.join("dist").exists(),
            "empty generated folder left behind"
        );
        // The ignored .env file was never touched.
        assert_eq!(fs::read(p.join(".env")).unwrap(), b"SECRET=1\n");
    }

    #[tokio::test]
    async fn redo_after_undo() {
        let (proj, _data, repo) = setup().await;
        let p = proj.path();
        let start = repo.snapshot("refs/fi/s/2/start", "start").await.unwrap();
        fs::write(p.join("src/a.rs"), "agent\n").unwrap();
        let end = repo.snapshot("refs/fi/s/2/end", "end").await.unwrap();
        let after = tree(p);
        let paths = repo
            .changed_paths(&start.commit, &end.commit)
            .await
            .unwrap();
        let pre = repo.snapshot("refs/fi/s/2/preundo", "pre").await.unwrap();
        repo.restore(&start.commit, &paths).await.unwrap();
        assert_ne!(tree(p), after);
        let r = repo.restore(&pre.commit, &paths).await.unwrap();
        assert!(r.verified);
        assert_eq!(tree(p), after);
    }

    #[tokio::test]
    async fn conflicts_are_detectable() {
        let (proj, _data, repo) = setup().await;
        let p = proj.path();
        let start = repo.snapshot("refs/fi/s/3/start", "start").await.unwrap();
        fs::write(p.join("src/a.rs"), "agent\n").unwrap();
        fs::write(p.join("src/b.rs"), "agent b\n").unwrap();
        let end = repo.snapshot("refs/fi/s/3/end", "end").await.unwrap();
        fs::write(p.join("src/a.rs"), "user changed it after\n").unwrap();
        let now = repo.snapshot("refs/fi/s/3/preundo", "now").await.unwrap();
        let turn: BTreeSet<String> = repo
            .changed_paths(&start.commit, &end.commit)
            .await
            .unwrap()
            .into_iter()
            .collect();
        let later = repo.changed_paths(&end.commit, &now.commit).await.unwrap();
        let conflicts: Vec<_> = later.into_iter().filter(|p| turn.contains(p)).collect();
        assert_eq!(conflicts, vec!["src/a.rs".to_string()]);
    }

    #[tokio::test]
    async fn nested_repos_and_big_files_are_listed() {
        let (proj, _data, repo) = setup().await;
        let p = proj.path();
        fs::create_dir_all(p.join("vendor/lib/.git")).unwrap();
        fs::write(p.join("vendor/lib/x.c"), "int x;").unwrap();
        let big = fs::File::create(p.join("big.bin")).unwrap();
        big.set_len(MAX_FILE_BYTES + 1).unwrap();
        let s = repo.snapshot("refs/fi/s/4/start", "s").await.unwrap();
        assert!(s.not_covered.contains(&NotCovered {
            path: "vendor/lib".into(),
            why: Uncovered::NestedRepo
        }));
        assert!(s.not_covered.contains(&NotCovered {
            path: "big.bin".into(),
            why: Uncovered::BigFile
        }));
        let files = repo
            .git(&["ls-tree", "-r", "--name-only", &s.commit])
            .await
            .unwrap();
        assert!(!files.contains("big.bin"));
        assert!(!files.contains("vendor"));
        assert!(!files.contains(".env"));
    }

    #[tokio::test]
    async fn prune_keeps_recent_turns() {
        let (_proj, _data, repo) = setup().await;
        for i in 0..(KEEP_TURNS + 3) {
            repo.snapshot(&format!("refs/fi/s/{i}/start"), "s")
                .await
                .unwrap();
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let dropped = repo.prune(now).await.unwrap();
        assert_eq!(dropped.len(), 3);
        assert_eq!(repo.turn_refs().await.unwrap().len(), KEEP_TURNS);
    }
}
