//! Project identity: a stable id and color generated from the folder path.

use std::path::Path;

/// Normalized key for a project folder: forward slashes, no trailing slash, and lower case on
/// Windows and macOS, where paths are case-insensitive by default.
pub fn path_key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let s = s.trim_end_matches('/').to_string();
    if cfg!(any(windows, target_os = "macos")) {
        s.to_lowercase()
    } else {
        s
    }
}

/// FNV-1a 64-bit. Stable across builds and platforms, unlike `DefaultHasher`.
pub fn fnv1a(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Short hex id for the project's app data folder.
pub fn project_id(path: &Path) -> String {
    format!("{:016x}", fnv1a(&path_key(path)))
}

/// Hue (0..360) for the project color. Never green, amber or red: those mean done, waiting
/// and failed. Allowed hues are 175..330 (teal, blue, indigo, violet, pink).
pub fn project_hue(path: &Path) -> u16 {
    const START: u64 = 175;
    const SPAN: u64 = 155;
    (START + fnv1a(&path_key(path)) % SPAN) as u16
}

/// Two-letter monogram from the folder name: "keeprun" -> "KE", "rig-sweep" -> "RS".
pub fn monogram(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let parts: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect();
    let mut out = String::new();
    match parts.as_slice() {
        [] => out.push('?'),
        [one] => out.extend(one.chars().take(2)),
        [a, b, ..] => {
            out.extend(a.chars().take(1));
            out.extend(b.chars().take(1));
        }
    }
    out.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hue_avoids_status_colors() {
        for i in 0..2000 {
            let h = project_hue(&PathBuf::from(format!("C:/work/project-{i}")));
            assert!((175..330).contains(&h), "{h}");
        }
    }

    #[test]
    fn same_path_same_id() {
        let a = project_id(&PathBuf::from("C:\\Work\\Keeprun\\"));
        let b = project_id(&PathBuf::from("C:/Work/Keeprun"));
        if cfg!(windows) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn monograms() {
        assert_eq!(monogram(&PathBuf::from("/x/keeprun")), "KE");
        assert_eq!(monogram(&PathBuf::from("/x/rig-sweep")), "RS");
        assert_eq!(monogram(&PathBuf::from("/x/focuside.dev")), "FD");
    }
}
