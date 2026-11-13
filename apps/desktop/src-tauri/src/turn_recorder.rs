//! Turn recording, shared by chat sessions (ACP) and agent panes (a TUI in a PTY).
//!
//! A turn is: claim the project, take a start snapshot and open the watch log, let the agent
//! work, then take an end snapshot, diff, record the turn, release the claim and prune old
//! snapshots. The recorder does not know where turn boundaries come from. Chat sessions use
//! the ACP prompt and its response. Panes use hook events (UserPromptSubmit, Stop,
//! StopFailure) when hooks run, and the Checkpoint button and pane exit otherwise.

use crate::project::Project;
use crate::turns::{now, TurnRecord};
use anyhow::{anyhow, Result};
use focuside_core::snapshot::{Changes, NotCovered, Snapshot};
use focuside_core::summary::{self, TurnSummary};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A new id with a prefix ("s" for sessions, "t" for turns): time in ms plus a counter.
pub fn new_id(prefix: &str) -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{prefix}{ms:x}{}", N.fetch_add(1, Ordering::Relaxed))
}

/// What the session knows about a turn when it ends.
pub struct TurnEnd<'a> {
    pub id: &'a str,
    pub start: &'a Snapshot,
    /// Paths the agent's own edit tools wrote (relative).
    pub edits: &'a BTreeSet<String>,
    /// Shell commands the agent ran.
    pub commands: Vec<String>,
    /// Paths the user saved in the FocusIDE viewer during the turn.
    pub your_edits: &'a BTreeSet<String>,
    /// This turn continues a paused one: update its record instead of adding one.
    pub continued: bool,
    /// The session reports the agent's edits and commands (ACP tool calls). A pane does not,
    /// so it cannot tell its own writes from writes made by something else.
    pub tools_known: bool,
    /// Plan steps for the summary (panes with hooks: the plan from ExitPlanMode).
    pub plan_steps: &'a [String],
    /// The agent's last message for the summary (panes with hooks: the Stop hook).
    pub last_message: Option<&'a str>,
}

/// What the end of a turn found on disk.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub changes: Changes,
    pub not_covered: Vec<NotCovered>,
    /// Every command of the record, including earlier parts of a continued turn.
    pub commands: Vec<String>,
    pub outside: bool,
    pub your_edits: BTreeSet<String>,
    pub ignored: Vec<String>,
    pub summary: TurnSummary,
}

impl Recorded {
    /// Adds the fields of the `turn_ended` event.
    pub fn write_to(&self, data: &mut Value) {
        data["changes"] = json!(self.changes);
        data["notCovered"] = json!(self.not_covered);
        data["commands"] = json!(self.commands);
        data["outside"] = json!(self.outside);
        data["yourEdits"] = json!(self.your_edits);
        data["ignoredChanged"] = json!(self.ignored);
        data["summary"] = json!(self.summary);
    }
}

/// "Includes changes from outside this session": files changed that the session's own tools
/// do not explain, in a turn without shell commands. Only a session that reports its tools can
/// tell; for a pane every change would look like an outside change, so the flag stays off.
pub fn outside_changes(
    tools_known: bool,
    commands: &[String],
    changes: &Changes,
    edits: &BTreeSet<String>,
    your_edits: &BTreeSet<String>,
) -> bool {
    tools_known
        && commands.is_empty()
        && changes
            .files
            .iter()
            .any(|f| !edits.contains(&f.path) && !your_edits.contains(&f.path))
}

/// Adds the turn's record, or updates it when the turn continues a paused one.
fn upsert(
    turns: &mut Vec<TurnRecord>,
    session_key: &str,
    end: &TurnEnd<'_>,
    end_commit: &str,
    r: &Recorded,
) {
    match turns.iter_mut().find(|t| t.id == end.id) {
        Some(rec) if end.continued => {
            rec.end = Some(end_commit.to_string());
            rec.changes = r.changes.clone();
            rec.commands.extend(end.commands.clone());
            rec.edits.extend(end.edits.iter().cloned());
            rec.your_edits.extend(end.your_edits.iter().cloned());
            rec.not_covered = r.not_covered.clone();
            rec.undone = None;
            for p in &r.ignored {
                if !rec.ignored_changed.contains(p) {
                    rec.ignored_changed.push(p.clone());
                }
            }
        }
        _ => turns.push(TurnRecord {
            id: end.id.to_string(),
            session_key: session_key.to_string(),
            start: end.start.commit.clone(),
            end: Some(end_commit.to_string()),
            changes: r.changes.clone(),
            not_covered: r.not_covered.clone(),
            commands: end.commands.clone(),
            edits: end.edits.iter().cloned().collect(),
            your_edits: end.your_edits.iter().cloned().collect(),
            ignored_changed: r.ignored.clone(),
            undone: None,
            created: now(),
        }),
    }
}

/// Records turns of one session in one project.
pub struct TurnRecorder {
    project: Arc<Project>,
    key: String,
}

impl TurnRecorder {
    pub fn new(project: Arc<Project>, session_key: &str) -> Self {
        Self {
            project,
            key: session_key.to_string(),
        }
    }

    /// Claims the project for a turn. False when another session holds it.
    pub fn claim(&self) -> bool {
        self.project.claim(&self.key)
    }

    pub fn release(&self) {
        self.project.release(&self.key);
    }

    /// True while this session holds the project.
    pub fn holds_claim(&self) -> bool {
        self.project.running_session().as_deref() == Some(self.key.as_str())
    }

    /// The start snapshot of a recorded turn (Continue after Pause keeps it).
    pub fn existing_start(&self, turn_id: &str) -> Option<Snapshot> {
        self.project
            .turns
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == turn_id)
            .map(|t| Snapshot {
                commit: t.start.clone(),
                not_covered: vec![],
            })
    }

    /// Opens the watch log and takes the start snapshot. Call after `claim`. On failure the
    /// claim is released.
    pub async fn start(&self, turn_id: &str) -> Result<Snapshot> {
        *self.project.watch_log.lock().unwrap() = Some(BTreeSet::new());
        let repo = &self.project.shadow;
        let snap = {
            let _g = repo.lock.lock().await;
            repo.snapshot(
                &format!("refs/fi/{}/{}/start", self.key, turn_id),
                "turn start",
            )
            .await
        };
        snap.map_err(|e| {
            self.release();
            anyhow!("snapshot: {e:#}")
        })
    }

    /// Ends a turn: end snapshot, diff, ignored files from the watch log, record. Then releases
    /// the claim and prunes old snapshots in the background, also when recording failed.
    pub async fn finish(&self, end: TurnEnd<'_>) -> Result<Recorded> {
        let res = self.record(&end).await;
        self.release();
        let repo = self.project.shadow.clone();
        tauri::async_runtime::spawn(async move {
            let _g = repo.lock.lock().await;
            if let Ok(dropped) = repo.prune(now()).await {
                if !dropped.is_empty() {
                    let _ = repo.gc().await;
                }
            }
        });
        res
    }

    async fn record(&self, end: &TurnEnd<'_>) -> Result<Recorded> {
        let repo = self.project.shadow.clone();
        let (end_snap, changes, ignored) = {
            let _g = repo.lock.lock().await;
            let end_snap = repo
                .snapshot(&format!("refs/fi/{}/{}/end", self.key, end.id), "turn end")
                .await?;
            let changes = repo.diff(&end.start.commit, &end_snap.commit).await?;
            // Files the watcher saw change that snapshots do not cover (ignored files).
            let seen: Vec<String> = self
                .project
                .watch_log
                .lock()
                .unwrap()
                .take()
                .unwrap_or_default()
                .into_iter()
                .filter(|p| !changes.files.iter().any(|f| &f.path == p))
                .collect();
            let mut ignored = repo.ignored(&seen).await.unwrap_or_default();
            ignored.truncate(20);
            (end_snap, changes, ignored)
        };
        let mut not_covered = end.start.not_covered.clone();
        for n in end_snap.not_covered {
            if !not_covered.contains(&n) {
                not_covered.push(n);
            }
        }
        let outside = outside_changes(
            end.tools_known,
            &end.commands,
            &changes,
            end.edits,
            end.your_edits,
        );
        let mut r = Recorded {
            changes,
            not_covered,
            commands: vec![],
            outside,
            your_edits: end.your_edits.clone(),
            ignored,
            summary: TurnSummary::default(),
        };
        {
            let mut turns = self.project.turns.lock().unwrap();
            upsert(&mut turns, &self.key, end, &end_snap.commit, &r);
            r.commands = turns
                .iter()
                .find(|t| t.id == end.id)
                .map(|t| t.commands.clone())
                .unwrap_or_default();
        }
        r.summary = summary::build(&r.changes, &r.commands, end.plan_steps, end.last_message);
        self.project.save_turns();
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focuside_core::snapshot::{FileChange, Status};

    fn changes(paths: &[&str]) -> Changes {
        Changes {
            files: paths
                .iter()
                .map(|p| FileChange {
                    path: p.to_string(),
                    old_path: None,
                    status: Status::Edit,
                    added: 1,
                    removed: 0,
                    binary: false,
                })
                .collect(),
            ..Default::default()
        }
    }

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn outside_rule() {
        let c = changes(&["a.txt", "b.txt"]);
        // Chat: a write the tools do not explain, no commands.
        assert!(outside_changes(true, &[], &c, &set(&["a.txt"]), &set(&[])));
        // Explained by tools and your edits.
        assert!(!outside_changes(
            true,
            &[],
            &c,
            &set(&["a.txt"]),
            &set(&["b.txt"])
        ));
        // A command could have written it.
        assert!(!outside_changes(
            true,
            &["npm i".into()],
            &c,
            &set(&[]),
            &set(&[])
        ));
        // Pane: no tool information, so never flagged.
        assert!(!outside_changes(false, &[], &c, &set(&[]), &set(&[])));
    }

    async fn project() -> (tempfile::TempDir, tempfile::TempDir, Arc<Project>) {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.txt"), "one\n").unwrap();
        let git = focuside_core::which::find("git").expect("git on PATH");
        let p = Project::open(root.path(), data.path(), &git).await.unwrap();
        (root, data, p)
    }

    #[tokio::test]
    async fn records_a_turn_and_releases_the_claim() {
        let (root, _data, p) = project().await;
        let r = TurnRecorder::new(p.clone(), "s1");
        let other = TurnRecorder::new(p.clone(), "s2");
        assert!(r.claim());
        assert!(!other.claim(), "a second session cannot claim the project");
        let start = r.start("t1").await.unwrap();
        assert!(r.holds_claim());

        std::fs::write(root.path().join("a.txt"), "two\n").unwrap();
        std::fs::write(root.path().join("b.txt"), "new\n").unwrap();
        let none = BTreeSet::new();
        let rec = r
            .finish(TurnEnd {
                id: "t1",
                start: &start,
                edits: &none,
                commands: vec![],
                your_edits: &none,
                continued: false,
                tools_known: false,
                plan_steps: &[],
                last_message: None,
            })
            .await
            .unwrap();
        let mut paths: Vec<_> = rec.changes.files.iter().map(|f| f.path.clone()).collect();
        paths.sort();
        assert_eq!(paths, vec!["a.txt", "b.txt"]);
        assert!(!rec.outside, "panes never flag outside changes");
        assert!(p.running_session().is_none(), "finish releases the claim");
        assert!(other.claim(), "the other session can claim it now");
        other.release();

        let turns = p.turns.lock().unwrap().clone();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].session_key, "s1");
        assert_eq!(turns[0].start, start.commit);
        assert!(turns[0].end.is_some());
        drop(turns);

        // The record is saved and undo works on it.
        let out = crate::turns::undo(&p, "t1", &[]).await.unwrap();
        assert!(out.exact);
        assert_eq!(
            std::fs::read_to_string(root.path().join("a.txt")).unwrap(),
            "one\n"
        );
        assert!(!root.path().join("b.txt").exists());
    }

    #[tokio::test]
    async fn continued_turn_updates_its_record() {
        let (root, _data, p) = project().await;
        let r = TurnRecorder::new(p.clone(), "s1");
        assert!(r.claim());
        let start = r.start("t1").await.unwrap();
        std::fs::write(root.path().join("a.txt"), "two\n").unwrap();
        let edits = set(&["a.txt"]);
        let none = BTreeSet::new();
        let first = r
            .finish(TurnEnd {
                id: "t1",
                start: &start,
                edits: &edits,
                commands: vec!["cargo build".into()],
                your_edits: &none,
                continued: false,
                tools_known: true,
                plan_steps: &[],
                last_message: None,
            })
            .await
            .unwrap();
        assert!(!first.outside);

        // Continue after Pause: same record, same start, commands add up.
        assert!(r.claim());
        let again = r.existing_start("t1").expect("start of t1");
        assert_eq!(again.commit, start.commit);
        std::fs::write(root.path().join("c.txt"), "x\n").unwrap();
        let rec = r
            .finish(TurnEnd {
                id: "t1",
                start: &again,
                edits: &none,
                commands: vec!["cargo test".into()],
                your_edits: &none,
                continued: true,
                tools_known: true,
                plan_steps: &[],
                last_message: None,
            })
            .await
            .unwrap();
        assert_eq!(rec.commands, vec!["cargo build", "cargo test"]);
        assert_eq!(rec.changes.files.len(), 2);
        let turns = p.turns.lock().unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].edits, vec!["a.txt"]);
    }

    #[tokio::test]
    async fn failed_snapshot_releases_the_claim() {
        let (_root, data, p) = project().await;
        let r = TurnRecorder::new(p.clone(), "s1");
        assert!(r.claim());
        // Break the shadow repo so the snapshot fails.
        let shadow = data.path().join("projects").join(&p.id).join("shadow.git");
        std::fs::remove_dir_all(&shadow).unwrap();
        assert!(r.start("t1").await.is_err());
        assert!(p.running_session().is_none());
    }
}
