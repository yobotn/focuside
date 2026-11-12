//! Plan card source: a deterministic parser for the `Steps:` / `Files:` / `Risk:` ending the
//! agent is asked to write (see `agent::PLAN_INSTRUCTION`), with a fallback for plans that do
//! not follow it.

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Change {
    Create,
    Edit,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanFile {
    pub change: Change,
    /// Relative to the project root, with forward slashes.
    pub path: String,
    pub reason: String,
    /// False when the plan edits or deletes a file that is not on disk.
    pub found: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedPlan {
    pub steps: Vec<String>,
    pub files: Vec<PlanFile>,
    pub risk: Option<String>,
    /// True when the agent skipped the short format and the card shows the first points.
    pub fallback: bool,
}

/// Parses plan text. Paths are not checked on disk; call `check_paths` for that.
pub fn parse(text: &str) -> ParsedPlan {
    let lines: Vec<&str> = text.lines().collect();
    let steps_at = lines.iter().rposition(|l| is_header(l, "steps"));
    let files_at = lines.iter().rposition(|l| is_header(l, "files"));
    let risk = lines.iter().rev().find_map(|l| {
        let s = strip_md(l.trim().trim_start_matches(['-', '*', '#', ' ']));
        let lower = s.to_lowercase();
        lower
            .strip_prefix("risk:")
            .or_else(|| lower.strip_prefix("risks:"))
            .map(|_| s[s.find(':').unwrap() + 1..].trim().to_string())
            .filter(|r| !r.is_empty())
    });

    if let Some(si) = steps_at {
        let end = files_at.filter(|f| *f > si).unwrap_or(lines.len());
        let steps: Vec<String> = lines[si + 1..end]
            .iter()
            .filter_map(|l| list_item(l))
            .filter(|s| !s.to_lowercase().starts_with("risk"))
            .collect();
        let files = files_at
            .map(|fi| {
                lines[fi + 1..]
                    .iter()
                    .take_while(|l| !is_header(l, "steps"))
                    .filter_map(|l| file_line(l))
                    .collect()
            })
            .unwrap_or_default();
        if !steps.is_empty() {
            return ParsedPlan {
                steps,
                files,
                risk,
                fallback: false,
            };
        }
    }

    // Fallback: the first 6 list items and the paths found in the text.
    let steps: Vec<String> = lines.iter().filter_map(|l| top_item(l)).take(6).collect();
    let mut files: Vec<PlanFile> = Vec::new();
    for p in find_paths(text) {
        if !files.iter().any(|f| f.path == p) {
            files.push(PlanFile {
                change: Change::Edit,
                path: p,
                reason: String::new(),
                found: true,
            });
        }
    }
    ParsedPlan {
        steps,
        files,
        risk,
        fallback: true,
    }
}

/// Normalizes paths against `root` and checks them on disk. In fallback mode, a path that is
/// not on disk becomes a create.
pub fn check_paths(plan: &mut ParsedPlan, root: &Path) {
    for f in &mut plan.files {
        f.path = normalize(&f.path, root);
        let exists = root.join(&f.path).exists();
        if plan.fallback && !exists {
            f.change = Change::Create;
        }
        f.found = match f.change {
            Change::Create => true,
            _ => exists,
        };
    }
}

/// Relative path with forward slashes. Absolute paths inside `root` become relative.
pub fn normalize(path: &str, root: &Path) -> String {
    let mut p = path.trim().trim_matches('`').replace('\\', "/");
    let root_s = root.to_string_lossy().replace('\\', "/");
    let root_s = root_s.trim_end_matches('/');
    if p.len() > root_s.len()
        && p[..root_s.len()].eq_ignore_ascii_case(root_s)
        && p.as_bytes()[root_s.len()] == b'/'
    {
        p = p[root_s.len() + 1..].to_string();
    }
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_string();
    }
    p
}

fn strip_md(s: &str) -> String {
    s.replace("**", "").replace("__", "")
}

/// "Steps:", "**Steps:**", "## Steps", "Files to change:" all count.
fn is_header(line: &str, word: &str) -> bool {
    let s = strip_md(line.trim().trim_start_matches('#').trim()).to_lowercase();
    let s = s.trim_end_matches(':').trim();
    s == word || (word == "files" && (s == "files to change" || s == "files changed"))
}

/// Text of a list item ("1. x", "1) x", "- x", "* x"), or None.
fn list_item(line: &str) -> Option<String> {
    let t = line.trim();
    let rest = if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        r
    } else {
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        let r = &t[digits..];
        r.strip_prefix(". ").or_else(|| r.strip_prefix(") "))?
    };
    let s = strip_md(rest).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// A list item that is not indented (top level), for the fallback.
fn top_item(line: &str) -> Option<String> {
    if line.starts_with("  ") || line.starts_with('\t') {
        return None;
    }
    list_item(line)
}

/// "create src/a.rs - reason", "- edit `src/b.rs`: reason".
fn file_line(line: &str) -> Option<PlanFile> {
    let t = line.trim();
    let t = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .unwrap_or(t);
    let t = strip_md(t);
    let t = t.trim();
    let (verb, rest) = t.split_once(char::is_whitespace)?;
    let change = match verb.to_lowercase().trim_end_matches(':') {
        "create" | "add" | "new" => Change::Create,
        "edit" | "modify" | "update" | "change" => Change::Edit,
        "delete" | "remove" => Change::Delete,
        _ => return None,
    };
    let rest = rest.trim();
    let (path, reason) = if let Some(stripped) = rest.strip_prefix('`') {
        let end = stripped.find('`')?;
        let after = stripped[end + 1..]
            .trim()
            .trim_start_matches(['-', ':', '\u{2013}', '\u{2014}'])
            .trim();
        (stripped[..end].to_string(), after.to_string())
    } else {
        let end = rest.find(" - ").or_else(|| rest.find(": "));
        match end {
            Some(e) => (
                rest[..e].trim().to_string(),
                rest[e + 3.min(rest.len() - e)..].trim().to_string(),
            ),
            None => (rest.to_string(), String::new()),
        }
    };
    let path = path.trim().trim_end_matches(':').to_string();
    if path.is_empty() || path.contains(' ') {
        return None;
    }
    Some(PlanFile {
        change,
        path,
        reason,
        found: true,
    })
}

/// Paths in the text: `code spans` or words with a slash or a file extension.
fn find_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |s: &str| {
        let s = s.trim_matches(|c: char| ",.;:()[]\"'<>*".contains(c));
        if looks_like_path(s) && !out.iter().any(|o: &String| o == s) {
            out.push(s.to_string());
        }
    };
    let mut in_code = false;
    let mut code = String::new();
    for ch in text.chars() {
        if ch == '`' {
            if in_code {
                push(&code);
                code.clear();
            }
            in_code = !in_code;
        } else if in_code {
            code.push(ch);
        }
    }
    for w in text.split_whitespace() {
        if !w.contains('`') {
            push(w);
        }
    }
    out
}

fn looks_like_path(s: &str) -> bool {
    if s.len() < 3 || s.contains("://") || s.contains(' ') || s.starts_with('-') {
        return false;
    }
    let last = s.rsplit('/').next().unwrap_or(s);
    let ext = last.rsplit_once('.');
    let has_ext = matches!(ext, Some((a, e)) if !a.is_empty() && (1..=6).contains(&e.len()) && e.chars().all(|c| c.is_ascii_alphanumeric()) && !e.chars().all(|c| c.is_ascii_digit()));
    let path_chars = s
        .chars()
        .all(|c| c.is_alphanumeric() || "/._-@+[]".contains(c));
    path_chars
        && (has_ext
            || (s.contains('/')
                && !s.ends_with('/')
                && s.matches('/').count() >= 1
                && s.chars().any(|c| c.is_alphabetic())
                && !s.starts_with('/')))
        && !matches!(last, "e.g" | "i.e" | "etc.")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "# Add a version flag\n\nSome context about the change.\n\n## Approach\n- use clap\n\nSteps:\n1. Add --version flag to the CLI parser\n2. Read the version from Cargo.toml at build time\n3. Add 1 test\n\nFiles:\nedit src/cli.rs - parse the flag\nedit `src/main.rs` - print and exit\ncreate tests/version.rs - new test\n\nRisk: none\n";

    #[test]
    fn parses_short_format() {
        let p = parse(GOOD);
        assert!(!p.fallback);
        assert_eq!(p.steps.len(), 3);
        assert_eq!(p.steps[0], "Add --version flag to the CLI parser");
        assert_eq!(p.files.len(), 3);
        assert_eq!(p.files[1].path, "src/main.rs");
        assert_eq!(p.files[1].reason, "print and exit");
        assert_eq!(p.files[2].change, Change::Create);
        assert_eq!(p.risk.as_deref(), Some("none"));
    }

    #[test]
    fn parses_markdown_headers_and_bullets() {
        let t = "**Steps:**\n- First thing\n- Second thing\n\n**Files:**\n- **edit** `a/b.ts`: because\n- delete old.txt - unused\n";
        let p = parse(t);
        assert!(!p.fallback);
        assert_eq!(p.steps, vec!["First thing", "Second thing"]);
        assert_eq!(p.files.len(), 2);
        assert_eq!(p.files[0].path, "a/b.ts");
        assert_eq!(p.files[0].reason, "because");
        assert_eq!(p.files[1].change, Change::Delete);
    }

    #[test]
    fn uses_last_steps_block() {
        let t = "Steps:\n1. old\n\nmore text\n\nSteps:\n1. new\nFiles:\nedit x.rs - y\n";
        let p = parse(t);
        assert_eq!(p.steps, vec!["new"]);
    }

    #[test]
    fn fallback_takes_first_six_and_paths() {
        let t = "I will:\n1. one in `src/a.rs`\n2. two\n   - nested\n3. three\n4. four\n5. five\n6. six\n7. seven touching lib/b.ts\nSee https://x.dev/a.html too.";
        let p = parse(t);
        assert!(p.fallback);
        assert_eq!(p.steps.len(), 6);
        assert_eq!(p.steps[1], "two");
        let paths: Vec<_> = p.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["src/a.rs", "lib/b.ts"]);
    }

    #[test]
    fn checks_and_normalizes_paths() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/cli.rs"), "").unwrap();
        let abs = dir.path().join("src/cli.rs").to_string_lossy().to_string();
        let mut p = parse(&format!(
            "Steps:\n1. x\nFiles:\nedit {abs} - a\nedit ./src/missing.rs - b\ncreate src/new.rs - c\n"
        ));
        check_paths(&mut p, dir.path());
        assert_eq!(p.files[0].path, "src/cli.rs");
        assert!(p.files[0].found);
        assert_eq!(p.files[1].path, "src/missing.rs");
        assert!(!p.files[1].found);
        assert!(p.files[2].found);
    }
}
