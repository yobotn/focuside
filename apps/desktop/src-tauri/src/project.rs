//! Open projects: identity, shadow snapshots, turn records and the one-running-turn rule.

use crate::turns::TurnRecord;
use anyhow::Result;
use focuside_core::project;
use focuside_core::snapshot::ShadowRepo;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub id: String,
    pub root: String,
    pub name: String,
    pub monogram: String,
    pub hue: u16,
    pub is_git: bool,
    pub branch: Option<String>,
}

pub struct Project {
    pub id: String,
    pub root: PathBuf,
    pub data_dir: PathBuf,
    pub shadow: Arc<ShadowRepo>,
    /// Session key of the session whose turn is running here, if any.
    pub running: Mutex<Option<String>>,
    pub turns: Mutex<Vec<TurnRecord>>,
    /// Paths the watcher saw change during the running turn.
    pub watch_log: Mutex<Option<BTreeSet<String>>>,
    /// Keeps the folder watcher alive.
    pub watcher: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

impl Project {
    pub async fn open(root: &Path, data_root: &Path, git: &Path) -> Result<Arc<Self>> {
        let id = project::project_id(root);
        let data_dir = data_root.join("projects").join(&id);
        let shadow = Arc::new(ShadowRepo::open(&data_dir, root, git).await?);
        let turns = crate::turns::load(&data_dir);
        Ok(Arc::new(Self {
            id,
            root: root.to_path_buf(),
            data_dir,
            shadow,
            running: Mutex::new(None),
            turns: Mutex::new(turns),
            watch_log: Mutex::new(None),
            watcher: Mutex::new(None),
        }))
    }

    pub fn info(&self) -> ProjectInfo {
        let name = self
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.root.to_string_lossy().to_string());
        ProjectInfo {
            id: self.id.clone(),
            root: self.root.to_string_lossy().to_string(),
            name,
            monogram: project::monogram(&self.root),
            hue: project::project_hue(&self.root),
            is_git: self.root.join(".git").exists(),
            branch: branch(&self.root),
        }
    }

    /// Claims the project for a running turn. Fails when another session is running here.
    pub fn claim(&self, session: &str) -> bool {
        let mut r = self.running.lock().unwrap();
        match r.as_deref() {
            Some(k) if k != session => false,
            _ => {
                *r = Some(session.to_string());
                true
            }
        }
    }

    pub fn release(&self, session: &str) {
        let mut r = self.running.lock().unwrap();
        if r.as_deref() == Some(session) {
            *r = None;
        }
    }

    pub fn running_session(&self) -> Option<String> {
        self.running.lock().unwrap().clone()
    }

    pub fn save_turns(&self) {
        crate::turns::save(&self.data_dir, &self.turns.lock().unwrap());
    }
}

/// Current branch from .git/HEAD, without running git. Worktrees and detached heads included.
pub fn branch(root: &Path) -> Option<String> {
    let dot_git = root.join(".git");
    let git_dir = if dot_git.is_file() {
        // Worktree or submodule: "gitdir: <path>".
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let p = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    } else {
        dot_git
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(r) = head.strip_prefix("ref: ") {
        Some(r.trim_start_matches("refs/heads/").to_string())
    } else {
        Some(head.chars().take(7).collect())
    }
}
