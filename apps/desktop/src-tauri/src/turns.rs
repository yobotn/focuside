//! Turn records and Undo turn: preview, undo with conflicts, redo, revert one file, and file
//! contents for the diff viewer.

use crate::project::Project;
use anyhow::{anyhow, Result};
use focuside_core::snapshot::{Changes, NotCovered};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoInfo {
    /// Snapshot taken right before the undo, so the undo itself can be undone.
    pub pre_undo: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnRecord {
    pub id: String,
    pub session_key: String,
    pub start: String,
    pub end: Option<String>,
    #[serde(default)]
    pub changes: Changes,
    #[serde(default)]
    pub not_covered: Vec<NotCovered>,
    /// Shell commands the agent ran in this turn.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Paths the agent's own edit tools wrote (relative).
    #[serde(default)]
    pub edits: Vec<String>,
    /// Paths the user saved in the FocusIDE viewer during the turn.
    #[serde(default)]
    pub your_edits: Vec<String>,
    /// Ignored files (not snapshotted) that changed during the turn.
    #[serde(default)]
    pub ignored_changed: Vec<String>,
    #[serde(default)]
    pub undone: Option<UndoInfo>,
    pub created: u64,
}

pub fn load(data_dir: &Path) -> Vec<TurnRecord> {
    std::fs::read(data_dir.join("turns.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, turns: &[TurnRecord]) {
    // Keep the records that still have snapshots (the shadow repo keeps 50 turns).
    let keep = &turns[turns
        .len()
        .saturating_sub(focuside_core::snapshot::KEEP_TURNS)..];
    if let Ok(b) = serde_json::to_vec_pretty(keep) {
        let tmp = data_dir.join("turns.json.tmp");
        if std::fs::write(&tmp, b).is_ok() {
            let _ = std::fs::rename(&tmp, data_dir.join("turns.json"));
        }
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn find(project: &Project, turn_id: &str) -> Result<TurnRecord> {
    project
        .turns
        .lock()
        .unwrap()
        .iter()
        .find(|t| t.id == turn_id)
        .cloned()
        .ok_or_else(|| anyhow!("This turn is no longer kept."))
}

fn update(project: &Project, rec: TurnRecord) {
    {
        let mut turns = project.turns.lock().unwrap();
        if let Some(t) = turns.iter_mut().find(|t| t.id == rec.id) {
            *t = rec;
        }
    }
    project.save_turns();
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoPreview {
    pub files: Vec<String>,
    /// Changed again after the turn ended (by the user or another tool).
    pub conflicts: Vec<String>,
    pub your_edits: Vec<String>,
    pub commands: Vec<String>,
    pub not_covered: Vec<NotCovered>,
    pub ignored_changed: Vec<String>,
}

pub async fn preview(project: &Project, turn_id: &str) -> Result<UndoPreview> {
    let rec = find(project, turn_id)?;
    let end = rec
        .end
        .clone()
        .ok_or_else(|| anyhow!("The turn is still running."))?;
    let repo = &project.shadow;
    let _g = repo.lock.lock().await;
    let files = repo.changed_paths(&rec.start, &end).await?;
    let now = repo
        .snapshot(
            &format!("refs/fi/{}/{}/now", rec.session_key, rec.id),
            "now",
        )
        .await?;
    let later: BTreeSet<String> = repo
        .changed_paths(&end, &now.commit)
        .await?
        .into_iter()
        .collect();
    let yours: BTreeSet<&String> = rec.your_edits.iter().collect();
    let conflicts = files
        .iter()
        .filter(|p| later.contains(*p) && !yours.contains(p))
        .cloned()
        .collect();
    let your_edits = files
        .iter()
        .filter(|p| yours.contains(p))
        .cloned()
        .collect();
    Ok(UndoPreview {
        files,
        conflicts,
        your_edits,
        commands: rec.commands.clone(),
        not_covered: rec.not_covered.clone(),
        ignored_changed: rec.ignored_changed.clone(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoOutcome {
    pub restored: Vec<String>,
    pub kept: Vec<String>,
    pub failed: Vec<(String, String)>,
    /// Every restored file matches the snapshot and nothing failed or was skipped.
    pub exact: bool,
}

/// Restores every file the turn changed, except conflicts the user keeps and their own
/// edits. `revert_anyway` lists conflicts to restore anyway.
pub async fn undo(
    project: &Project,
    turn_id: &str,
    revert_anyway: &[String],
) -> Result<UndoOutcome> {
    let mut rec = find(project, turn_id)?;
    let end = rec
        .end
        .clone()
        .ok_or_else(|| anyhow!("The turn is still running."))?;
    let repo = &project.shadow;
    let _g = repo.lock.lock().await;
    let files = repo.changed_paths(&rec.start, &end).await?;
    let pre = repo
        .snapshot(
            &format!("refs/fi/{}/{}/preundo", rec.session_key, rec.id),
            "before undo",
        )
        .await?;
    let later: BTreeSet<String> = repo
        .changed_paths(&end, &pre.commit)
        .await?
        .into_iter()
        .collect();
    let yours: BTreeSet<&String> = rec.your_edits.iter().collect();
    let anyway: BTreeSet<&String> = revert_anyway.iter().collect();
    let (restore, kept): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|p| !yours.contains(p) && (!later.contains(p) || anyway.contains(p)));
    let r = repo.restore(&rec.start, &restore).await?;
    let exact = r.verified
        && kept.is_empty()
        && rec.not_covered.is_empty()
        && rec.ignored_changed.is_empty();
    rec.undone = Some(UndoInfo {
        pre_undo: pre.commit,
        paths: restore.clone(),
    });
    update(project, rec);
    Ok(UndoOutcome {
        restored: r.restored,
        kept,
        failed: r.failed,
        exact,
    })
}

/// Puts back the files from right before the undo.
pub async fn redo(project: &Project, turn_id: &str) -> Result<UndoOutcome> {
    let mut rec = find(project, turn_id)?;
    let info = rec
        .undone
        .clone()
        .ok_or_else(|| anyhow!("This turn was not undone."))?;
    let repo = &project.shadow;
    let _g = repo.lock.lock().await;
    let r = repo.restore(&info.pre_undo, &info.paths).await?;
    rec.undone = None;
    let exact = r.verified;
    update(project, rec);
    Ok(UndoOutcome {
        restored: r.restored,
        kept: vec![],
        failed: r.failed,
        exact,
    })
}

/// Restores one file (and the old path of a rename) to how it was before the turn.
pub async fn revert_file(project: &Project, turn_id: &str, path: &str) -> Result<UndoOutcome> {
    let rec = find(project, turn_id)?;
    let mut paths = vec![path.to_string()];
    if let Some(old) = rec
        .changes
        .files
        .iter()
        .find(|f| f.path == path)
        .and_then(|f| f.old_path.clone())
    {
        paths.push(old);
    }
    let repo = &project.shadow;
    let _g = repo.lock.lock().await;
    let r = repo.restore(&rec.start, &paths).await?;
    Ok(UndoOutcome {
        exact: r.verified,
        restored: r.restored,
        kept: vec![],
        failed: r.failed,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersions {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub binary: bool,
}

/// File content before and after the turn, for the diff viewer.
pub async fn versions(project: &Project, turn_id: &str, path: &str) -> Result<FileVersions> {
    let rec = find(project, turn_id)?;
    let end = rec.end.clone().unwrap_or_else(|| rec.start.clone());
    let old_path = rec
        .changes
        .files
        .iter()
        .find(|f| f.path == path)
        .and_then(|f| f.old_path.clone())
        .unwrap_or_else(|| path.to_string());
    let repo = &project.shadow;
    let before = repo.read_file(&rec.start, &old_path).await?;
    let after = repo.read_file(&end, path).await?;
    let is_bin = |b: &Option<Vec<u8>>| {
        b.as_ref()
            .map(|b| b.iter().take(8000).any(|c| *c == 0) || std::str::from_utf8(b).is_err())
            .unwrap_or(false)
    };
    let binary = is_bin(&before) || is_bin(&after);
    let text = |b: Option<Vec<u8>>| b.map(|b| String::from_utf8_lossy(&b).to_string());
    Ok(FileVersions {
        path: path.to_string(),
        before: if binary { None } else { text(before) },
        after: if binary { None } else { text(after) },
        binary,
    })
}
