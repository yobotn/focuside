//! Project files: lazy directory listing, the full file list for pickers, reading and writing
//! with size and binary checks, and search in files.

use anyhow::{bail, Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::{BinaryDetection, SearcherBuilder};
use regex::{NoExpand, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
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

/// Largest image the viewer shows.
pub const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;
/// Largest PDF the viewer shows.
pub const MAX_PDF_BYTES: u64 = 100 * 1024 * 1024;

/// MIME type of an image or PDF the viewer can show, from the file extension.
pub fn media_mime(path: &str) -> Option<&'static str> {
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())?;
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        _ => return None,
    })
}

/// Size limit for a media file, by MIME type.
pub fn media_limit(mime: &str) -> u64 {
    if mime == "application/pdf" {
        MAX_PDF_BYTES
    } else {
        MAX_IMAGE_BYTES
    }
}

/// Reads an image or PDF inside the project for the viewer. Fails for other types and for
/// files above the limit of their type.
pub fn read_media(root: &Path, rel_path: &str) -> Result<Vec<u8>> {
    let Some(mime) = media_mime(rel_path) else {
        bail!("This file type cannot be shown.");
    };
    let p = resolve(root, rel_path)?;
    let size = std::fs::metadata(&p)
        .with_context(|| format!("read {}", p.display()))?
        .len();
    if size > media_limit(mime) {
        bail!("This file is too large to show.");
    }
    std::fs::read(&p).with_context(|| format!("read {}", p.display()))
}

pub fn write_file(root: &Path, rel_path: &str, content: &str) -> Result<()> {
    let p = resolve(root, rel_path)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, content).with_context(|| format!("write {}", p.display()))
}

/// What to look for in files. Search and replace in files share it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    pub query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    /// Comma separated globs. When set, only files that match one are searched.
    pub include: String,
    /// Comma separated globs of files and folders to skip.
    pub exclude: String,
}

impl SearchQuery {
    /// The regular expression for the query: escaped unless `regex`, with word boundaries for
    /// `whole_word` (only next to word characters, so "-x" still finds "a -x b").
    pub fn pattern(&self) -> String {
        if self.regex {
            return if self.whole_word {
                format!(r"\b(?:{})\b", self.query)
            } else {
                self.query.clone()
            };
        }
        let mut p = regex_escape(&self.query);
        if self.whole_word {
            let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
            if word(self.query.chars().next()) {
                p.insert_str(0, r"\b");
            }
            if word(self.query.chars().last()) {
                p.push_str(r"\b");
            }
        }
        p
    }

    /// The compiled pattern, used for match ranges and for replacing.
    pub fn regex(&self) -> Result<Regex> {
        RegexBuilder::new(&self.pattern())
            .case_insensitive(!self.case_sensitive)
            .build()
            .context("invalid search pattern")
    }
}

/// Include and exclude globs, VS Code style: a glob without a slash matches a name at any
/// depth ("*.ts", "node_modules"), one with a slash matches from the project root ("src/**").
/// A glob that matches a folder matches everything in it.
pub struct PathFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

fn glob_set(list: &str) -> Result<Option<GlobSet>> {
    let mut b = GlobSetBuilder::new();
    let mut any = false;
    for raw in list.split(',') {
        let g = raw.trim().replace('\\', "/");
        let g = g.trim_start_matches("./").trim_end_matches('/');
        if g.is_empty() {
            continue;
        }
        let base = match g.strip_prefix('/') {
            Some(anchored) => anchored.to_string(),
            None if g.contains('/') => g.to_string(),
            None => format!("**/{g}"),
        };
        for pat in [base.clone(), format!("{base}/**")] {
            let glob = GlobBuilder::new(&pat)
                .literal_separator(true)
                .case_insensitive(cfg!(windows))
                .build()
                .with_context(|| format!("invalid glob: {g}"))?;
            b.add(glob);
        }
        any = true;
    }
    Ok(if any { Some(b.build()?) } else { None })
}

impl PathFilter {
    pub fn new(include: &str, exclude: &str) -> Result<Self> {
        Ok(Self {
            include: glob_set(include)?,
            exclude: glob_set(exclude)?,
        })
    }

    /// True when a file (relative path) is searched.
    pub fn keeps_file(&self, rel: &str) -> bool {
        if self.exclude.as_ref().is_some_and(|x| x.is_match(rel)) {
            return false;
        }
        self.include.as_ref().is_none_or(|x| x.is_match(rel))
    }
}

/// The files a search looks at, in path order: what the ignore rules keep, filtered by the
/// globs, up to `MAX_VIEW_BYTES * 4` bytes each. Returns full and relative paths.
fn search_files(root: &Path, filter: &PathFilter) -> Vec<(PathBuf, String)> {
    let mut walker = ignore::WalkBuilder::new(root);
    let base = root.to_path_buf();
    let exclude = filter.exclude.clone();
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(move |e| {
            if e.file_name() == ".git" {
                return false;
            }
            let dir = e.depth() > 0 && e.file_type().is_some_and(|t| t.is_dir());
            !(dir
                && exclude
                    .as_ref()
                    .is_some_and(|x| x.is_match(rel(&base, e.path()))))
        });
    let mut out = Vec::new();
    for e in walker.build().flatten() {
        if !e.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = rel(root, e.path());
        if !filter.keeps_file(&path) {
            continue;
        }
        if e.metadata()
            .map(|m| m.len() > MAX_VIEW_BYTES * 4)
            .unwrap_or(true)
        {
            continue;
        }
        out.push((e.path().to_path_buf(), path));
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub line: u64,
    /// UTF-16 column of the first match in the whole line, for the editor.
    pub col: u32,
    /// The part of the line around the match: no leading space, at most 300 characters.
    pub text: String,
    /// True when `text` starts after the start of the line (the match is far to the right).
    pub cut: bool,
    /// The matches in `text`, as UTF-16 offsets (start, end).
    pub ranges: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub truncated: bool,
}

const PREVIEW_CHARS: usize = 300;
const PREVIEW_BEFORE: usize = 40;

/// The preview of one line: the text, the match ranges in it (UTF-16) and whether the start
/// of the line was cut.
fn preview(line: &str, matches: &[(usize, usize)]) -> (String, Vec<[u32; 2]>, bool) {
    let lead = line.len() - line.trim_start().len();
    let first = matches.first().map(|m| m.0).unwrap_or(lead);
    let mut start = lead.min(first);
    if first > start {
        let before: Vec<usize> = line[start..first].char_indices().map(|(i, _)| i).collect();
        if before.len() > PREVIEW_BEFORE {
            start += before[before.len() - PREVIEW_BEFORE];
        }
    }
    let end = line[start..]
        .char_indices()
        .nth(PREVIEW_CHARS)
        .map(|(i, _)| start + i)
        .unwrap_or(line.len());
    let u16 = |b: usize| line[start..b].encode_utf16().count() as u32;
    let ranges = matches
        .iter()
        .filter(|(s, e)| *s >= start && *s < end && e > s)
        .map(|(s, e)| [u16(*s), u16((*e).min(end))])
        .collect();
    (line[start..end].to_string(), ranges, start > lead)
}

/// Searches text in every file the ignore rules and the globs keep. Binary files are skipped.
/// At most 200 lines per file and `max` lines in all.
pub fn search(root: &Path, q: &SearchQuery, max: usize) -> Result<SearchResult> {
    if q.query.is_empty() {
        return Ok(SearchResult {
            hits: vec![],
            truncated: false,
        });
    }
    let filter = PathFilter::new(&q.include, &q.exclude)?;
    let re = q.regex()?;
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(!q.case_sensitive)
        .line_terminator(Some(b'\n'))
        .build(&q.pattern())
        .context("invalid search pattern")?;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(0))
        .line_number(true)
        .build();
    let mut hits = Vec::new();
    let mut truncated = false;
    'files: for (full, path) in search_files(root, &filter) {
        let mut file_hits = Vec::new();
        let _ = searcher.search_path(
            &matcher,
            &full,
            UTF8(|line, text| {
                let t = text.trim_end_matches(['\r', '\n']);
                let found: Vec<(usize, usize)> = re
                    .find_iter(t)
                    .filter(|m| !m.is_empty())
                    .map(|m| (m.start(), m.end()))
                    .collect();
                let col = found
                    .first()
                    .map(|m| t[..m.0].encode_utf16().count() as u32)
                    .unwrap_or(0);
                let (text, ranges, cut) = preview(t, &found);
                file_hits.push(SearchHit {
                    path: path.clone(),
                    line,
                    col,
                    text,
                    cut,
                    ranges,
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

/// A file with the number of matches in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileMatches {
    pub path: String,
    pub count: usize,
}

/// Every file with a match and the number of matches in it, for the preview before Replace
/// all. At most `max` files.
pub fn count_matches(root: &Path, q: &SearchQuery, max: usize) -> Result<Vec<FileMatches>> {
    if q.query.is_empty() {
        return Ok(vec![]);
    }
    let filter = PathFilter::new(&q.include, &q.exclude)?;
    let re = q.regex()?;
    let mut out = Vec::new();
    for (full, path) in search_files(root, &filter) {
        let Some(text) = read_text(&full) else {
            continue;
        };
        let count: usize = lines(&text)
            .map(|(body, _)| re.find_iter(body).filter(|m| !m.is_empty()).count())
            .sum();
        if count > 0 {
            out.push(FileMatches { path, count });
            if out.len() >= max {
                break;
            }
        }
    }
    Ok(out)
}

/// A text file's content, or None for binary, non UTF-8 and too large files.
pub fn read_text(full: &Path) -> Option<String> {
    let meta = std::fs::metadata(full).ok()?;
    if meta.len() > MAX_VIEW_BYTES * 4 {
        return None;
    }
    let bytes = std::fs::read(full).ok()?;
    if bytes.iter().take(8000).any(|b| *b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// Lines with their line break ("\r\n", "\n", or "" for the last line).
fn lines(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.split_inclusive('\n').map(|seg| {
        if let Some(body) = seg.strip_suffix("\r\n") {
            (body, "\r\n")
        } else if let Some(body) = seg.strip_suffix('\n') {
            (body, "\n")
        } else {
            (seg, "")
        }
    })
}

/// Replaces every match in `text` line by line, like the search finds them: a match never
/// spans lines and line breaks stay as they are. With `regex`, `$1` and `${name}` in `with`
/// insert groups. Returns the new text and the number of replaced matches.
pub fn replace_text(text: &str, q: &SearchQuery, re: &Regex, with: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    for (body, brk) in lines(text) {
        let n = re.find_iter(body).filter(|m| !m.is_empty()).count();
        if n == 0 {
            out.push_str(body);
        } else {
            count += n;
            let replaced = if q.regex {
                re.replace_all(body, with)
            } else {
                re.replace_all(body, NoExpand(with))
            };
            out.push_str(&replaced);
        }
        out.push_str(brk);
    }
    (out, count)
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

    fn q(query: &str) -> SearchQuery {
        SearchQuery {
            query: query.into(),
            ..Default::default()
        }
    }

    #[test]
    fn search_finds_text() {
        let d = proj();
        let r = search(d.path(), &q("hello"), 100).unwrap();
        let paths: Vec<_> = r.hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(paths, vec!["src/deep/x.ts", "src/main.rs"]);
        assert!(!paths.iter().any(|p| p.starts_with("target")));
        let cs = search(
            d.path(),
            &SearchQuery {
                case_sensitive: true,
                ..q("hello")
            },
            100,
        )
        .unwrap();
        assert_eq!(cs.hits.len(), 1);
        assert_eq!(cs.hits[0].line, 1);
        assert_eq!(cs.hits[0].ranges, vec![[13, 18]]);
        let re = search(
            d.path(),
            &SearchQuery {
                regex: true,
                ..q(r"print\w+!")
            },
            100,
        )
        .unwrap();
        assert_eq!(re.hits[0].line, 2);
        // Leading space is not part of the preview; the column counts it.
        assert_eq!(re.hits[0].text, "println!(\"Hello\");");
        assert_eq!(re.hits[0].col, 4);
        assert_eq!(re.hits[0].ranges, vec![[0, 8]]);
        assert!(search(
            d.path(),
            &SearchQuery {
                regex: true,
                ..q("(")
            },
            10
        )
        .is_err());
    }

    #[test]
    fn search_whole_word_and_globs() {
        let d = proj();
        fs::write(d.path().join("src/w.txt"), "hello helloworld -x a-x\n").unwrap();
        let word = |query: &str| SearchQuery {
            whole_word: true,
            ..q(query)
        };
        let r = search(d.path(), &word("hello"), 100).unwrap();
        let w = r.hits.iter().find(|h| h.path == "src/w.txt").unwrap();
        assert_eq!(w.ranges, vec![[0, 5]]);
        // Next to a non-word character there is no boundary to check.
        let r = search(d.path(), &word("-x"), 100).unwrap();
        assert_eq!(r.hits[0].ranges, vec![[17, 19], [21, 23]]);
        let only = |include: &str, exclude: &str| {
            let r = search(
                d.path(),
                &SearchQuery {
                    include: include.into(),
                    exclude: exclude.into(),
                    ..q("hello")
                },
                100,
            )
            .unwrap();
            let mut p: Vec<String> = r.hits.iter().map(|h| h.path.clone()).collect();
            p.dedup();
            p
        };
        assert_eq!(only("*.ts", ""), vec!["src/deep/x.ts"]);
        assert_eq!(only("src/deep", ""), vec!["src/deep/x.ts"]);
        assert_eq!(only("", "deep, *.txt"), vec!["src/main.rs"]);
        assert_eq!(
            only("src/**/*.rs, *.txt", ""),
            vec!["src/main.rs", "src/w.txt"]
        );
        // Globs never bring back ignored files.
        assert!(only("target/**", "").is_empty());
    }

    #[test]
    fn preview_cuts_long_lines() {
        let line = format!("{}needle end", "a".repeat(100));
        let (text, ranges, cut) = preview(&line, &[(100, 106)]);
        assert!(cut);
        assert!(text.starts_with(&"a".repeat(40)));
        assert_eq!(ranges, vec![[40, 46]]);
        // UTF-16 offsets: one emoji is two code units.
        let (_, ranges, cut) = preview("\u{1F600}ab", &[(4, 5)]);
        assert!(!cut);
        assert_eq!(ranges, vec![[2, 3]]);
    }

    #[test]
    fn replace_keeps_line_breaks() {
        let lit = SearchQuery {
            case_sensitive: true,
            ..q("a.b")
        };
        let re = lit.regex().unwrap();
        let (out, n) = replace_text("a.b axb\r\nA.B a.b\nlast a.b", &lit, &re, "$1");
        assert_eq!(out, "$1 axb\r\nA.B $1\nlast $1");
        assert_eq!(n, 3);
        let rx = SearchQuery {
            regex: true,
            ..q(r"(\w+)@(\w+)")
        };
        let re = rx.regex().unwrap();
        let (out, n) = replace_text("x@y\nnone\n", &rx, &re, "$2 at $1");
        assert_eq!(out, "y at x\nnone\n");
        assert_eq!(n, 1);
        let d = proj();
        let c = count_matches(d.path(), &q("hello"), 10).unwrap();
        assert_eq!(
            c,
            vec![
                FileMatches {
                    path: "src/deep/x.ts".into(),
                    count: 1
                },
                FileMatches {
                    path: "src/main.rs".into(),
                    count: 1
                }
            ]
        );
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
        assert_eq!(media_mime("a/B.PNG"), Some("image/png"));
        assert_eq!(media_mime("doc.pdf"), Some("application/pdf"));
        assert_eq!(media_mime("src/main.rs"), None);
        assert_eq!(media_mime("noext"), None);
        fs::write(d.path().join("pic.png"), [0x89, b'P', b'N', b'G']).unwrap();
        assert_eq!(read_media(d.path(), "pic.png").unwrap().len(), 4);
        assert!(read_media(d.path(), "src/main.rs").is_err());
        assert!(read_media(d.path(), "../pic.png").is_err());
        assert!(read_media(d.path(), "gone.png").is_err());
        write_file(d.path(), "new/dir/f.txt", "hi").unwrap();
        assert_eq!(
            fs::read_to_string(d.path().join("new/dir/f.txt")).unwrap(),
            "hi"
        );
    }
}
