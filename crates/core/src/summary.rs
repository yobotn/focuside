//! Turn summary without a model call: built from the disk diff, the commands the agent ran,
//! the plan steps and, in panes with hooks, the agent's last message. The UI turns it into
//! sentences (strings live in the UI's en.ts).

use crate::snapshot::{Changes, Status};
use serde::Serialize;

/// How many commands and plan steps the summary lists. Counts stay exact.
pub const MAX_COMMANDS: usize = 5;
pub const MAX_STEPS: usize = 8;
/// The longest message excerpt, in characters.
pub const MAX_EXCERPT: usize = 280;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSummary {
    pub files: usize,
    pub created: usize,
    pub deleted: usize,
    pub added: u32,
    pub removed: u32,
    /// Files in generated folders (counted, not listed).
    pub generated: usize,
    pub command_count: usize,
    /// The first commands, without repeats.
    pub commands: Vec<String>,
    pub plan_steps: Vec<String>,
    pub plan_step_count: usize,
    /// The start of the agent's last message: first paragraph, markdown marks removed.
    pub excerpt: Option<String>,
}

/// Builds the summary. `last_message` is the agent's final answer when known (Stop hook).
pub fn build(
    changes: &Changes,
    commands: &[String],
    plan_steps: &[String],
    last_message: Option<&str>,
) -> TurnSummary {
    let mut listed: Vec<String> = Vec::new();
    for c in commands {
        let c = one_line(c);
        if !c.is_empty() && !listed.contains(&c) && listed.len() < MAX_COMMANDS {
            listed.push(c);
        }
    }
    TurnSummary {
        files: changes.files.len(),
        created: changes
            .files
            .iter()
            .filter(|f| f.status == Status::Create)
            .count(),
        deleted: changes
            .files
            .iter()
            .filter(|f| f.status == Status::Delete)
            .count(),
        added: changes.added,
        removed: changes.removed,
        generated: changes.generated.iter().map(|g| g.count).sum(),
        command_count: commands.iter().filter(|c| !c.trim().is_empty()).count(),
        commands: listed,
        plan_steps: plan_steps.iter().take(MAX_STEPS).cloned().collect(),
        plan_step_count: plan_steps.len(),
        excerpt: last_message.and_then(excerpt),
    }
}

/// A command on one line, cut to 120 characters.
fn one_line(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    cut_words(&s, 120)
}

/// Cuts on a word boundary and adds "..." when text was dropped.
fn cut_words(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max - 3).collect();
    let head = match head.rfind(' ') {
        Some(i) if i > max / 2 => head[..i].to_string(),
        _ => head,
    };
    format!("{}...", head.trim_end())
}

/// The first paragraph of a message as plain text: no headings marks, list marks, emphasis or
/// code fences. None for an empty message.
pub fn excerpt(msg: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut in_fence = false;
    for line in msg.lines() {
        let t = line.trim();
        if t.starts_with("```") {
            in_fence = !in_fence;
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if in_fence {
            continue;
        }
        if t.is_empty() {
            if lines.is_empty() {
                continue;
            }
            break;
        }
        let t = t.trim_start_matches(['#', '>', '-', '*', ' ']);
        lines.push(t.replace("**", "").replace('`', ""));
    }
    let text = lines.join(" ");
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(cut_words(text, MAX_EXCERPT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::FileChange;

    fn change(path: &str, status: Status, added: u32, removed: u32) -> FileChange {
        FileChange {
            path: path.into(),
            old_path: None,
            status,
            added,
            removed,
            binary: false,
        }
    }

    #[test]
    fn counts_files_lines_and_commands() {
        let changes = Changes {
            files: vec![
                change("a.rs", Status::Edit, 10, 2),
                change("b.rs", Status::Create, 5, 0),
                change("c.rs", Status::Delete, 0, 7),
            ],
            added: 15,
            removed: 9,
            ..Default::default()
        };
        let cmds: Vec<String> = ["cargo test", "cargo  test", "npm run\nlint", " "]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let s = build(&changes, &cmds, &["Read".into(), "Fix".into()], None);
        assert_eq!((s.files, s.created, s.deleted), (3, 1, 1));
        assert_eq!((s.added, s.removed), (15, 9));
        assert_eq!(s.command_count, 3);
        assert_eq!(s.commands, vec!["cargo test", "npm run lint"]);
        assert_eq!(s.plan_step_count, 2);
        assert_eq!(s.excerpt, None);
    }

    #[test]
    fn excerpt_takes_the_first_paragraph() {
        let msg = "## Done\n\nI fixed the **parser** in `plan.rs`.\nTests pass.\n\nDetails:\n- one";
        assert_eq!(excerpt(msg).unwrap(), "Done");
        let msg = "I fixed the **parser** in `plan.rs`.\nTests pass.\n\nMore.";
        assert_eq!(
            excerpt(msg).unwrap(),
            "I fixed the parser in plan.rs. Tests pass."
        );
        assert_eq!(excerpt("  \n\n"), None);
        assert_eq!(excerpt("```\ncode\n```\nAfter."), Some("After.".into()));
        let long = "word ".repeat(200);
        let e = excerpt(&long).unwrap();
        assert!(e.chars().count() <= MAX_EXCERPT && e.ends_with("..."));
    }
}
