//! Hunks of one file in a turn, and Undo for one hunk. A hunk is one run of changed lines
//! between the file at the start of the turn (`before`) and at its end (`after`), with no
//! context lines. Undo change writes the `before` lines of one hunk back into the file on disk,
//! but only while the file still has the agent's lines there, with the line above and the line
//! below unchanged. Otherwise the file is left as it is: the same rule as Undo in Replace all
//! (replace.rs), which never overwrites later edits.

use crate::files;
use crate::snapshot::ShadowRepo;
use anyhow::{bail, Result};
use serde::Serialize;
use similar::{capture_diff_slices, Algorithm, DiffOp};
use std::ops::Range;
use std::path::Path;

/// The error text when a hunk changed after the turn. The UI shows its own sentence for it.
pub const CHANGED_SINCE: &str = "changedSince";

/// Lines with their line break ("\r\n", "\n", or none for the last line).
pub fn split_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HunkState {
    /// The file on disk still has the agent's lines here. Undo change is possible.
    Agent,
    /// The file on disk has the lines from before the turn here.
    Undone,
    /// The lines changed after the turn.
    Changed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    /// First line in `before` (0-based) and the number of lines there.
    pub old_start: usize,
    pub old_len: usize,
    /// First line in `after` (0-based) and the number of lines there.
    pub new_start: usize,
    pub new_len: usize,
    pub state: HunkState,
}

/// The file on disk compared with the turn's end.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Current {
    /// Exactly as the agent left it.
    Agent,
    Changed,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub binary: bool,
    pub before: Option<String>,
    pub after: Option<String>,
    pub hunks: Vec<Hunk>,
    pub current: Current,
    /// Undo change works per hunk: both versions are text. A new or deleted file only has
    /// Undo file.
    pub hunk_undo: bool,
}

fn ops(a: &[&str], b: &[&str]) -> Vec<DiffOp> {
    capture_diff_slices(Algorithm::Myers, a, b)
}

/// The changed line ranges between `a` and `b`, as (range in a, range in b).
fn ranges(a: &[&str], b: &[&str]) -> Vec<(Range<usize>, Range<usize>)> {
    let mut out: Vec<(Range<usize>, Range<usize>)> = vec![];
    for op in ops(a, b) {
        let (tag, ra, rb) = op.as_tag_tuple();
        if tag == similar::DiffTag::Equal {
            continue;
        }
        match out.last_mut() {
            Some((la, lb)) if la.end == ra.start && lb.end == rb.start => {
                la.end = ra.end;
                lb.end = rb.end;
            }
            _ => out.push((ra, rb)),
        }
    }
    out
}

/// Where lines `s..e` of `old` are in `new`, given the diff ops from `old` to `new`. Only when
/// those lines, the line above and the line below are unchanged and still next to each other.
/// Returns the first line in `new`.
fn map_range(ops: &[DiffOp], old_len: usize, new_len: usize, s: usize, e: usize) -> Option<usize> {
    let lo = s.saturating_sub(1);
    let hi = (e + 1).min(old_len);
    if lo >= hi {
        // Nothing around the range to check (an empty file): only when both are empty.
        return (old_len == 0 && new_len == 0).then_some(0);
    }
    let mut map = vec![None; old_len];
    for op in ops {
        if let DiffOp::Equal {
            old_index,
            new_index,
            len,
        } = *op
        {
            for k in 0..len {
                map[old_index + k] = Some(new_index + k);
            }
        }
    }
    let mut prev: Option<usize> = None;
    for slot in &map[lo..hi] {
        let j = (*slot)?;
        if let Some(p) = prev {
            if j != p + 1 {
                return None;
            }
        }
        prev = Some(j);
    }
    let first = map[lo]?;
    Some(if s == lo { first } else { first + 1 })
}

/// The hunks of a change, with their state against the file on disk.
pub fn hunks(before: &str, after: &str, current: Option<&str>) -> Vec<Hunk> {
    let b = split_lines(before);
    let a = split_lines(after);
    let c = current.map(split_lines);
    let ac = c.as_ref().map(|c| ops(&a, c));
    let bc = c.as_ref().map(|c| ops(&b, c));
    let cl = c.as_ref().map(|c| c.len()).unwrap_or(0);
    ranges(&b, &a)
        .into_iter()
        .map(|(rb, ra)| {
            let state = match (&ac, &bc) {
                (Some(ac), _) if map_range(ac, a.len(), cl, ra.start, ra.end).is_some() => {
                    HunkState::Agent
                }
                (_, Some(bc)) if map_range(bc, b.len(), cl, rb.start, rb.end).is_some() => {
                    HunkState::Undone
                }
                _ => HunkState::Changed,
            };
            Hunk {
                old_start: rb.start,
                old_len: rb.len(),
                new_start: ra.start,
                new_len: ra.len(),
                state,
            }
        })
        .collect()
}

/// The text of `current` with hunk `index` of the change put back to `before`. Fails with
/// CHANGED_SINCE when the file no longer has the agent's lines there.
pub fn undo_hunk_text(before: &str, after: &str, current: &str, index: usize) -> Result<String> {
    let b = split_lines(before);
    let a = split_lines(after);
    let c = split_lines(current);
    let Some((rb, ra)) = ranges(&b, &a).into_iter().nth(index) else {
        bail!("This change is no longer in the turn.");
    };
    let Some(at) = map_range(&ops(&a, &c), a.len(), c.len(), ra.start, ra.end) else {
        bail!(CHANGED_SINCE);
    };
    let mut out = String::with_capacity(current.len());
    out.extend(c[..at].iter().copied());
    out.extend(b[rb].iter().copied());
    out.extend(c[at + ra.len()..].iter().copied());
    Ok(out)
}

fn is_binary(b: &[u8]) -> bool {
    b.iter().take(8000).any(|c| *c == 0) || std::str::from_utf8(b).is_err()
}

fn text(b: Option<Vec<u8>>) -> Option<String> {
    b.map(|b| String::from_utf8_lossy(&b).to_string())
}

/// One file of a turn: `start` and `end` are the turn's snapshots. A renamed file is read from
/// `old_path` at the start.
pub struct TurnFile<'a> {
    pub start: &'a str,
    pub end: &'a str,
    pub path: &'a str,
    pub old_path: Option<&'a str>,
}

impl TurnFile<'_> {
    async fn versions(&self, repo: &ShadowRepo) -> Result<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let before = repo
            .read_file(self.start, self.old_path.unwrap_or(self.path))
            .await?;
        let after = repo.read_file(self.end, self.path).await?;
        Ok((before, after))
    }
}

fn read_current(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    let full = files::resolve(root, path)?;
    match std::fs::read(&full) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn build(
    f: &TurnFile<'_>,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
    now: Option<Vec<u8>>,
) -> FileDiff {
    let binary =
        before.as_deref().is_some_and(is_binary) || after.as_deref().is_some_and(is_binary);
    let current = match &now {
        None => Current::Missing,
        Some(n) if Some(n) == after.as_ref() => Current::Agent,
        Some(_) => Current::Changed,
    };
    let now_text = now
        .filter(|n| !is_binary(n))
        .map(|n| String::from_utf8_lossy(&n).to_string());
    let (before, after) = if binary {
        (None, None)
    } else {
        (text(before), text(after))
    };
    let hunk_undo = !binary && before.is_some() && after.is_some();
    let hunks = match (&before, &after) {
        (Some(b), Some(a)) => hunks(b, a, now_text.as_deref()),
        (None, Some(a)) => hunks("", a, now_text.as_deref()),
        (Some(b), None) => hunks(b, "", now_text.as_deref().or(Some(""))),
        (None, None) => vec![],
    };
    FileDiff {
        path: f.path.to_string(),
        old_path: f.old_path.map(String::from),
        binary,
        before,
        after,
        hunks,
        current,
        hunk_undo,
    }
}

/// Both versions of a file in a turn, its hunks and their state against the file on disk.
pub async fn file_diff(repo: &ShadowRepo, f: &TurnFile<'_>) -> Result<FileDiff> {
    let (before, after) = f.versions(repo).await?;
    let now = read_current(repo.work_tree(), f.path)?;
    Ok(build(f, before, after, now))
}

/// Undo change: puts hunk `index` back to how it was before the turn, in the file on disk.
/// Fails with CHANGED_SINCE when those lines changed after the turn. Returns the new diff.
pub async fn undo_hunk(repo: &ShadowRepo, f: &TurnFile<'_>, index: usize) -> Result<FileDiff> {
    let _g = repo.lock.lock().await;
    let (before, after) = f.versions(repo).await?;
    let (Some(b), Some(a)) = (&before, &after) else {
        bail!("Use Undo file for a new or deleted file.");
    };
    if is_binary(b) || is_binary(a) {
        bail!("Binary files have no changes to undo one by one.");
    }
    let Some(now) = read_current(repo.work_tree(), f.path)? else {
        bail!(CHANGED_SINCE);
    };
    let Ok(now_text) = String::from_utf8(now) else {
        bail!(CHANGED_SINCE);
    };
    let (b, a) = (String::from_utf8_lossy(b), String::from_utf8_lossy(a));
    let next = undo_hunk_text(&b, &a, &now_text, index)?;
    std::fs::write(files::resolve(repo.work_tree(), f.path)?, next.as_bytes())?;
    Ok(build(f, before, after, Some(next.into_bytes())))
}

/// True when the file on disk is exactly as the agent left it at the end of the turn (or both
/// are missing).
pub async fn unchanged_since(repo: &ShadowRepo, end: &str, path: &str) -> Result<bool> {
    let after = repo.read_file(end, path).await?;
    let now = read_current(repo.work_tree(), path)?;
    Ok(after == now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const BEFORE: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\n";
    const AFTER: &str = "one\nTWO\nthree\nfour\nfive\nsix\nseven\neight\n";

    #[test]
    fn hunks_are_runs_of_changed_lines() {
        let h = hunks(BEFORE, AFTER, Some(AFTER));
        assert_eq!(h.len(), 2);
        assert_eq!(
            (h[0].old_start, h[0].old_len, h[0].new_start, h[0].new_len),
            (1, 1, 1, 1)
        );
        assert_eq!(
            (h[1].old_start, h[1].old_len, h[1].new_start, h[1].new_len),
            (7, 0, 7, 1)
        );
        assert!(h.iter().all(|x| x.state == HunkState::Agent));
    }

    #[test]
    fn undo_one_hunk_keeps_the_rest() {
        let out = undo_hunk_text(BEFORE, AFTER, AFTER, 1).unwrap();
        assert_eq!(out, "one\nTWO\nthree\nfour\nfive\nsix\nseven\n");
        let h = hunks(BEFORE, AFTER, Some(&out));
        assert_eq!(h[0].state, HunkState::Agent);
        assert_eq!(h[1].state, HunkState::Undone);
        // Then the other one: the file is as before the turn.
        let out = undo_hunk_text(BEFORE, AFTER, &out, 0).unwrap();
        assert_eq!(out, BEFORE);
    }

    #[test]
    fn edits_elsewhere_do_not_block_undo() {
        // The user added a line at the top after the turn. The hunk moved one line down.
        let now = format!("zero\n{AFTER}");
        let out = undo_hunk_text(BEFORE, AFTER, &now, 0).unwrap();
        assert_eq!(out, format!("zero\n{}", AFTER.replace("TWO", "two")));
    }

    #[test]
    fn a_changed_hunk_is_left_alone() {
        let now = AFTER.replace("TWO", "Two (mine)");
        let err = undo_hunk_text(BEFORE, AFTER, &now, 0).unwrap_err();
        assert_eq!(err.to_string(), CHANGED_SINCE);
        assert_eq!(
            hunks(BEFORE, AFTER, Some(&now))[0].state,
            HunkState::Changed
        );
        // A line added right next to the hunk also counts as a change there.
        let now = AFTER.replace("TWO\n", "TWO\nnew\n");
        assert_eq!(
            undo_hunk_text(BEFORE, AFTER, &now, 0)
                .unwrap_err()
                .to_string(),
            CHANGED_SINCE
        );
    }

    #[test]
    fn crlf_and_last_line_without_break() {
        let before = "a\r\nb\r\nc";
        let after = "a\r\nB\r\nc";
        let out = undo_hunk_text(before, after, after, 0).unwrap();
        assert_eq!(out, before);
        let before = "x\ny";
        let after = "x\ny\nz";
        assert_eq!(undo_hunk_text(before, after, after, 0).unwrap(), before);
    }

    fn git() -> std::path::PathBuf {
        crate::which::find("git").expect("git on PATH")
    }

    #[tokio::test]
    async fn undo_hunk_against_a_shadow_repo() {
        let proj = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let p = proj.path();
        fs::write(p.join("a.txt"), BEFORE.replace('\n', "\r\n")).unwrap();
        fs::write(p.join("new.txt"), "").unwrap();
        fs::remove_file(p.join("new.txt")).unwrap();
        let repo = ShadowRepo::open(data.path(), p, &git()).await.unwrap();
        let start = repo.snapshot("refs/fi/s/1/start", "start").await.unwrap();
        // The agent's turn.
        fs::write(p.join("a.txt"), AFTER.replace('\n', "\r\n")).unwrap();
        fs::write(p.join("new.txt"), "fresh\n").unwrap();
        let end = repo.snapshot("refs/fi/s/1/end", "end").await.unwrap();
        let f = TurnFile {
            start: &start.commit,
            end: &end.commit,
            path: "a.txt",
            old_path: None,
        };
        let d = file_diff(&repo, &f).await.unwrap();
        assert!(d.hunk_undo);
        assert_eq!(d.current, Current::Agent);
        assert_eq!(d.hunks.len(), 2);
        assert!(unchanged_since(&repo, &end.commit, "a.txt").await.unwrap());

        // The user edits the first hunk after the turn: only the second can be undone.
        fs::write(
            p.join("a.txt"),
            AFTER.replace("TWO", "Two (mine)").replace('\n', "\r\n"),
        )
        .unwrap();
        assert!(!unchanged_since(&repo, &end.commit, "a.txt").await.unwrap());
        let err = undo_hunk(&repo, &f, 0).await.unwrap_err();
        assert_eq!(err.to_string(), CHANGED_SINCE);
        let d = undo_hunk(&repo, &f, 1).await.unwrap();
        assert_eq!(d.current, Current::Changed);
        assert_eq!(d.hunks[0].state, HunkState::Changed);
        assert_eq!(d.hunks[1].state, HunkState::Undone);
        assert_eq!(
            fs::read_to_string(p.join("a.txt")).unwrap(),
            BEFORE.replace("two", "Two (mine)").replace('\n', "\r\n")
        );

        // A new file has no Undo change, only Undo file.
        let n = TurnFile {
            start: &start.commit,
            end: &end.commit,
            path: "new.txt",
            old_path: None,
        };
        let d = file_diff(&repo, &n).await.unwrap();
        assert!(!d.hunk_undo);
        assert_eq!(d.before, None);
        assert!(undo_hunk(&repo, &n, 0).await.is_err());
        assert_eq!(fs::read_to_string(p.join("new.txt")).unwrap(), "fresh\n");
    }
}
