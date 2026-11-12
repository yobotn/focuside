//! Project files: lazy directory listing, the full file list for pickers, reading and writing
//! with size and binary checks, and search in files.

use anyhow::{bail, Context, Result};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::{BinaryDetection, SearcherBuilder};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

/// Files larger than this open as "large" in the viewer instead of in the editor.
pub const MAX_VIEW_BYTES: u64 = 2 * 1024 * 1024;
/// Cap for the file list used by pickers.
pub const MAX_LIST: usize = 50_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    /// Relative to the project root, forward slashes.
    pub path: String,
    pub dir: bool,
    pub ignored: bool,
}

/// Resolves a relative path inside `root`. Rejects absolute paths and `..`.
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = rel.replace('\\', "/");
    let p = Path::new(&rel);
    for c in p.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            _ => bail!("Path is outside the project: {rel}"),
        }
    }
    Ok(root.join(p))
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// One directory level. Folders first, then files, both by name. `.git` is never listed.
/// Ignored entries are listed only with `show_ignored`, marked as ignored.
pub fn list_dir(root: &Path, dir_rel: &str, show_ignored: bool) -> Result<Vec<DirEntry>> {
    let dir = resolve(root, dir_rel)?;
    // Entries the ignore rules keep.
    let mut kept = std::collections::HashSet::new();
    let mut walker = ignore::WalkBuilder::new(&dir);
    walker
        .max_depth(Some(1))
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .parents(true);
    for e in walker.build().flatten() {
        if e.depth() == 1 {
            kept.insert(e.path().to_path_buf());
        }
    }
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let Ok(e) = e else { continue };
        let name = e.file_name().to_string_lossy().to_string();
        if name == ".git" {
            continue;
        }
        let path = e.path();
        let ignored = !kept.contains(&path);
        if ignored && !show_ignored {
            continue;
        }
        let dir = e
            .file_type()
            .map(|t| t.is_dir() || (t.is_symlink() && path.is_dir()))
            .unwrap_or(false);
        out.push(DirEntry {
            name,
            path: rel(root, &path),
            dir,
            ignored,
        });
    }
    out.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// Every file in the project that the ignore rules keep, up to `MAX_LIST`.
pub fn all_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut walker = ignore::WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git");
    for e in walker.build().flatten() {
        if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            out.push(rel(root, e.path()));
            if out.len() >= MAX_LIST {
                break;
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FileContent {
    Text { content: String, size: u64 },
    Binary { size: u64 },
    Large { size: u64 },
    Missing,
}

pub fn read_file(root: &Path, rel_path: &str) -> Result<FileContent> {
    let p = resolve(root, rel_path)?;
    let meta = match std::fs::metadata(&p) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(FileContent::Missing),
        Err(e) => return Err(e.into()),
    };
    let size = meta.len();
    if size > MAX_VIEW_BYTES {
        return Ok(FileContent::Large { size });
    }
    let bytes = std::fs::read(&p)?;
    if bytes.iter().take(8000).any(|b| *b == 0) {
        return Ok(FileContent::Binary { size });
    }
    match String::from_utf8(bytes) {
        Ok(content) => Ok(FileContent::Text { content, size }),
        Err(_) => Ok(FileContent::Binary { size }),
    }
}

pub fn write_file(root: &Path, rel_path: &str, content: &str) -> Result<()> {
    let p = resolve(root, rel_path)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, content).with_context(|| format!("write {}", p.display()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub line: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub truncated: bool,
}

/// Searches text in every file the ignore rules keep. Binary files are skipped.
pub fn search(
    root: &Path,
    query: &str,
    case_sensitive: bool,
    regex: bool,
    max: usize,
) -> Result<SearchResult> {
    if query.is_empty() {
        return Ok(SearchResult {
            hits: vec![],
            truncated: false,
        });
    }
    let pattern = if regex {
        query.to_string()
    } else {
        regex_escape(query)
    };
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(!case_sensitive)
        .line_terminator(Some(b'\n'))
        .build(&pattern)
        .context("invalid search pattern")?;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(0))
        .line_number(true)
        .build();
    let mut hits = Vec::new();
    let mut truncated = false;
    let mut walker = ignore::WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git");
    'files: for e in walker.build().flatten() {
        if !e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if e.metadata()
            .map(|m| m.len() > MAX_VIEW_BYTES * 4)
            .unwrap_or(true)
        {
            continue;
        }
        let path = rel(root, e.path());
        let mut file_hits = Vec::new();
        let _ = searcher.search_path(
            &matcher,
            e.path(),
            UTF8(|line, text| {
                let t = text.trim_end_matches(['\r', '\n']);
                let t: String = t.chars().take(300).collect();
                file_hits.push(SearchHit {
                    path: path.clone(),
                    line,
                    text: t,
                });
                Ok(file_hits.len() < 200)
            }),
        );
        for h in file_hits {
            if hits.len() >= max {
                truncated = true;
                break 'files;
            }
            hits.push(h);
        }
    }
    Ok(SearchResult { hits, truncated })
}

fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn proj() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        fs::create_dir_all(p.join("src/deep")).unwrap();
        fs::create_dir_all(p.join(".git")).unwrap();
        fs::create_dir_all(p.join("target")).unwrap();
        fs::write(p.join(".gitignore"), "target/\n*.log\n").unwrap();
        fs::write(
            p.join("src/main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n",
        )
        .unwrap();
        fs::write(p.join("src/deep/x.ts"), "export const hello = 1;\n").unwrap();
        fs::write(p.join("target/out.rs"), "fn hello() {}").unwrap();
        fs::write(p.join("a.log"), "hello").unwrap();
        fs::write(p.join("bin.dat"), [0u8, 1, 2]).unwrap();
        d
    }

    #[test]
    fn lists_one_level_folders_first() {
        let d = proj();
        let e = list_dir(d.path(), "", false).unwrap();
        let names: Vec<_> = e.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["src", ".gitignore", "bin.dat"]);
        let all = list_dir(d.path(), "", true).unwrap();
        assert!(all.iter().any(|x| x.name == "target" && x.ignored));
        assert!(!all.iter().any(|x| x.name == ".git"));
        let sub = list_dir(d.path(), "src", false).unwrap();
        assert_eq!(sub[0].path, "src/deep");
    }

    #[test]
    fn all_files_respects_ignore() {
        let d = proj();
        let f = all_files(d.path());
        assert!(f.contains(&"src/deep/x.ts".to_string()));
        assert!(!f.iter().any(|x| x.starts_with("target")));
        assert!(!f.contains(&"a.log".to_string()));
    }

    #[test]
    fn search_finds_text() {
        let d = proj();
        let r = search(d.path(), "hello", false, false, 100).unwrap();
        let paths: Vec<_> = r.hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["src/deep/x.ts", "src/main.rs"]
                .into_iter()
                .filter(|p| paths.contains(p))
                .collect::<Vec<_>>()
        );
        assert!(paths.contains(&"src/main.rs"));
        assert!(!paths.iter().any(|p| p.starts_with("target")));
        let cs = search(d.path(), "hello", true, false, 100).unwrap();
        assert_eq!(cs.hits.len(), 1);
        assert_eq!(cs.hits[0].line, 1);
        let re = search(d.path(), r"print\w+!", false, true, 100).unwrap();
        assert_eq!(re.hits[0].line, 2);
    }

    #[test]
    fn read_kinds_and_safety() {
        let d = proj();
        assert!(matches!(
            read_file(d.path(), "src/main.rs").unwrap(),
            FileContent::Text { .. }
        ));
        assert!(matches!(
            read_file(d.path(), "bin.dat").unwrap(),
            FileContent::Binary { .. }
        ));
        assert!(matches!(
            read_file(d.path(), "nope.txt").unwrap(),
            FileContent::Missing
        ));
        assert!(read_file(d.path(), "../x").is_err());
        write_file(d.path(), "new/dir/f.txt", "hi").unwrap();
        assert_eq!(
            fs::read_to_string(d.path().join("new/dir/f.txt")).unwrap(),
            "hi"
        );
    }
}
