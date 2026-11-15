//! Replace in files, with Undo. Before it writes, Replace all takes a snapshot of the project
//! in the shadow repo (`refs/fi/replace/<id>/before`, see snapshot.rs). Undo writes each file
//! back from that snapshot, but only while the file still has the text Replace all wrote. A
//! file the snapshot does not hold exactly (a nested repo, a skipped folder) keeps its old
//! text in memory instead.

use crate::files::{self, FileMatches, SearchQuery};
use crate::snapshot::ShadowRepo;
use anyhow::Result;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// What Undo needs for one replaced file.
#[derive(Debug, Clone)]
struct Replaced {
    path: String,
    /// Hash of the text Replace all wrote. Undo skips the file when the text changed since.
    after: u64,
    /// The old text, when the snapshot does not hold it exactly.
    backup: Option<String>,
}

/// One Replace all, kept in memory for Undo.
#[derive(Debug, Clone)]
pub struct Batch {
    commit: Option<String>,
    files: Vec<Replaced>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceOutcome {
    pub files: Vec<FileMatches>,
    pub failed: Vec<(String, String)>,
    /// False when the snapshot failed and no file has a copy in memory: Undo is not possible.
    pub undo: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoReplaceOutcome {
    pub restored: Vec<String>,
    /// Files that changed after Replace all. Undo leaves them as they are.
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>,
}

fn hash(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// Replaces every match of `q` with `with` in `paths` (relative). `id` names the snapshot.
/// Returns the batch for Undo and what changed.
pub async fn replace_in_files(
    root: &Path,
    repo: &ShadowRepo,
    id: &str,
    q: &SearchQuery,
    with: &str,
    paths: &[String],
) -> Result<(Batch, ReplaceOutcome)> {
    let re = q.regex()?;
    let _g = repo.lock.lock().await;
    let commit = repo
        .snapshot(
            &format!("refs/fi/replace/{id}/before"),
            "before replace in files",
        )
        .await
        .map(|s| s.commit)
        .map_err(|e| tracing::warn!("replace snapshot: {e:#}"))
        .ok();
    let mut batch = Batch {
        commit: commit.clone(),
        files: vec![],
    };
    let mut out = ReplaceOutcome::default();
    for rel in paths {
        let full = match files::resolve(root, rel) {
            Ok(f) => f,
            Err(e) => {
                out.failed.push((rel.clone(), format!("{e:#}")));
                continue;
            }
        };
        let Some(text) = files::read_text(&full) else {
            out.failed
                .push((rel.clone(), "Not a text file, or too large.".into()));
            continue;
        };
        let (new, count) = files::replace_text(&text, q, &re, with);
        if count == 0 {
            continue;
        }
        // Undo reads the old text from the snapshot only when the snapshot has it exactly.
        let held = match &commit {
            Some(c) => repo.read_file(c, rel).await.ok().flatten(),
            None => None,
        };
        let backup = (held.as_deref() != Some(text.as_bytes())).then(|| text.clone());
        if let Err(e) = std::fs::write(&full, &new) {
            out.failed.push((rel.clone(), e.to_string()));
            continue;
        }
        batch.files.push(Replaced {
            path: rel.clone(),
            after: hash(&new),
            backup,
        });
        out.files.push(FileMatches {
            path: rel.clone(),
            count,
        });
    }
    out.undo = !batch.files.is_empty()
        && (batch.commit.is_some() || batch.files.iter().all(|f| f.backup.is_some()));
    Ok((batch, out))
}

/// Puts back the text from before Replace all, in every file that still has the replaced text.
pub async fn undo_replace(
    root: &Path,
    repo: &ShadowRepo,
    batch: &Batch,
) -> Result<UndoReplaceOutcome> {
    let _g = repo.lock.lock().await;
    let mut out = UndoReplaceOutcome::default();
    for f in &batch.files {
        let full = files::resolve(root, &f.path)?;
        let now = files::read_text(&full);
        if now.as_deref().map(hash) != Some(f.after) {
            out.skipped.push(f.path.clone());
            continue;
        }
        let old = match (&f.backup, &batch.commit) {
            (Some(text), _) => Some(text.clone().into_bytes()),
            (None, Some(c)) => repo.read_file(c, &f.path).await.ok().flatten(),
            (None, None) => None,
        };
        let Some(old) = old else {
            out.failed
                .push((f.path.clone(), "The old text is not available.".into()));
            continue;
        };
        match std::fs::write(&full, old) {
            Ok(()) => out.restored.push(f.path.clone()),
            Err(e) => out.failed.push((f.path.clone(), e.to_string())),
        }
    }
    Ok(out)
}

impl Batch {
    /// The replaced files, relative.
    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn replace_and_undo() {
        let proj = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let p = proj.path();
        fs::create_dir_all(p.join("src")).unwrap();
        fs::write(p.join("src/a.rs"), "let old = 1;\r\nold();\r\n").unwrap();
        fs::write(p.join("src/b.rs"), "old\n").unwrap();
        fs::write(p.join("src/c.rs"), "nothing\n").unwrap();
        let git = crate::which::find("git").expect("git on PATH");
        let repo = ShadowRepo::open(data.path(), p, &git).await.unwrap();
        let q = SearchQuery {
            query: "old".into(),
            whole_word: true,
            ..Default::default()
        };
        let paths: Vec<String> = files::count_matches(p, &q, 100)
            .unwrap()
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(paths, vec!["src/a.rs", "src/b.rs"]);
        let (batch, out) = replace_in_files(p, &repo, "t1", &q, "new", &paths)
            .await
            .unwrap();
        assert!(out.undo);
        assert!(out.failed.is_empty());
        assert_eq!(out.files.iter().map(|f| f.count).sum::<usize>(), 3);
        assert_eq!(
            fs::read_to_string(p.join("src/a.rs")).unwrap(),
            "let new = 1;\r\nnew();\r\n"
        );
        // The snapshot holds the old text, so nothing is kept in memory.
        assert!(batch.files.iter().all(|f| f.backup.is_none()));
        // b.rs changed after Replace all: Undo leaves it.
        fs::write(p.join("src/b.rs"), "mine\n").unwrap();
        let undo = undo_replace(p, &repo, &batch).await.unwrap();
        assert_eq!(undo.restored, vec!["src/a.rs"]);
        assert_eq!(undo.skipped, vec!["src/b.rs"]);
        assert_eq!(
            fs::read(p.join("src/a.rs")).unwrap(),
            b"let old = 1;\r\nold();\r\n"
        );
        assert_eq!(fs::read_to_string(p.join("src/b.rs")).unwrap(), "mine\n");
        assert_eq!(batch.paths(), vec!["src/a.rs", "src/b.rs"]);
    }
}
