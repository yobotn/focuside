//! Per-session data the UI keeps between runs: plan versions and the prompts of a pane session
//! (prompt text, Claude's last answer, the turn and its changes). One JSON file per session in
//! `<app data>/projects/<id>/sessions/<session id>.json`. The owner approved keeping prompts
//! and answers there (2026-11-14). FocusIDE never writes them into the project and never sends
//! them anywhere.

use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The file of one session, or None when `id` is not a plain id (letters, digits, `-`, `_`).
pub fn path(project_data: &Path, id: &str) -> Option<PathBuf> {
    let ok = !id.is_empty()
        && id.len() <= 120
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then(|| project_data.join("sessions").join(format!("{id}.json")))
}

/// The saved data, or null when there is none or it does not parse.
pub fn load(project_data: &Path, id: &str) -> Value {
    path(project_data, id)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null)
}

/// Writes the data: a temporary file, then a rename.
pub fn save(project_data: &Path, id: &str, value: &Value) -> Result<()> {
    let p = path(project_data, id).ok_or_else(|| anyhow::anyhow!("bad session id"))?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(value)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn saves_and_loads_by_plain_id() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load(d.path(), "pane-s1"), Value::Null);
        save(d.path(), "pane-s1", &json!({ "prompts": [1] })).unwrap();
        assert_eq!(load(d.path(), "pane-s1")["prompts"][0], 1);
        for bad in ["", "../x", "a/b", r"a\b", "a.json", "a b"] {
            assert!(path(d.path(), bad).is_none(), "{bad:?}");
            assert!(save(d.path(), bad, &json!({})).is_err());
        }
        assert!(path(d.path(), "0b2c-4d_e").is_some());
    }
}
