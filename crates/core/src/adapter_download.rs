//! Downloads the pinned claude-agent-acp release on first use. FocusIDE does not ship the adapter
//! in its installer. When the user starts a Claude chat session and agrees, this module fetches
//! the upstream release archive from GitHub, checks its pinned SHA-256, and writes only the
//! adapter executable to app data. The Claude CLI in the same archive is never written: FocusIDE
//! runs the user's own `claude`. The same pin and asset names are in scripts/fetch-adapter.mjs,
//! which the dev setup uses.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The pinned upstream version. When you change it, update the hashes and sizes below,
/// scripts/fetch-adapter.mjs and THIRD_PARTY_NOTICES.md.
pub const VERSION: &str = "0.106.0";
/// The upstream GitHub repository.
pub const REPO: &str = "agentclientprotocol/claude-agent-acp";
const NAME: &str = "claude-agent-acp";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    TarGz,
}

/// One release asset of the pinned version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asset {
    /// `std::env::consts::OS` value.
    pub os: &'static str,
    /// `std::env::consts::ARCH` value.
    pub arch: &'static str,
    /// Suffix after `claude-agent-acp-` in the release file name.
    pub file: &'static str,
    pub format: Format,
    /// SHA-256 of the whole archive, lowercase hex.
    pub sha256: &'static str,
    /// Archive size in bytes.
    pub size: u64,
}

pub const ASSETS: &[Asset] = &[
    Asset {
        os: "windows",
        arch: "x86_64",
        file: "windows-x64.zip",
        format: Format::Zip,
        sha256: "5de09fe80ac78260eb56964b92e698bd4c467092f0c88e042438a8e05974903c",
        size: 219_220_423,
    },
    Asset {
        os: "windows",
        arch: "aarch64",
        file: "windows-arm64.zip",
        format: Format::Zip,
        sha256: "ffc36b8e00b23942f8214e1f70606a77c1c4d90ef3a366422a093f728e59454c",
        size: 208_117_866,
    },
    Asset {
        os: "macos",
        arch: "aarch64",
        file: "darwin-arm64.zip",
        format: Format::Zip,
        sha256: "446c5ff8303c1fb0caaa5b047566ce4f88c5a70d188ed798d236cfc68f875f9e",
        size: 186_542_729,
    },
    Asset {
        os: "macos",
        arch: "x86_64",
        file: "darwin-x64.zip",
        format: Format::Zip,
        sha256: "a6fbfc9814e176615d68fd58dda1887ae2b9ff00a18c9e8b9d75352ce7bd0fba",
        size: 196_623_507,
    },
    Asset {
        os: "linux",
        arch: "x86_64",
        file: "linux-x64.tar.gz",
        format: Format::TarGz,
        sha256: "7d27db341f2a09bd64e54465de8409c624c92827d19f2065f32c599377727c6e",
        size: 209_931_437,
    },
    Asset {
        os: "linux",
        arch: "aarch64",
        file: "linux-arm64.tar.gz",
        format: Format::TarGz,
        sha256: "04390d95545916b21df886e92ff24538f7d71a86f657556704747f07de18422d",
        size: 209_499_544,
    },
];

/// The asset for an OS and architecture, as named by `std::env::consts`.
pub fn asset_for(os: &str, arch: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|a| a.os == os && a.arch == arch)
}

/// The asset for the machine FocusIDE runs on.
pub fn current() -> Option<&'static Asset> {
    asset_for(std::env::consts::OS, std::env::consts::ARCH)
}

impl Asset {
    pub fn file_name(&self) -> String {
        format!("{NAME}-{}", self.file)
    }

    pub fn url(&self) -> String {
        format!(
            "https://github.com/{REPO}/releases/download/v{VERSION}/{}",
            self.file_name()
        )
    }

    /// The adapter's file name inside the archive and on disk.
    pub fn exe_name(&self) -> &'static str {
        if self.os == "windows" {
            "claude-agent-acp.exe"
        } else {
            NAME
        }
    }
}

/// The folder for one adapter version: `<data_root>/adapters/claude-agent-acp-<version>`.
/// A new pin gets a new folder, so a version bump downloads a fresh copy.
pub fn install_dir(data_root: &Path, version: &str) -> PathBuf {
    data_root.join("adapters").join(format!("{NAME}-{version}"))
}

/// Where the pinned adapter lives for this machine, present or not.
pub fn adapter_path(data_root: &Path) -> Option<PathBuf> {
    let asset = current()?;
    Some(install_dir(data_root, VERSION).join(asset.exe_name()))
}

static DATA_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Set once by the desktop app at start: its app data folder.
pub fn set_data_root(root: PathBuf) {
    let _ = DATA_ROOT.set(root);
}

pub fn data_root() -> Option<&'static Path> {
    DATA_ROOT.get().map(|p| p.as_path())
}

/// The downloaded adapter, when it is present.
pub fn installed() -> Option<PathBuf> {
    adapter_path(data_root()?).filter(|p| p.is_file())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

/// Fails unless `actual` equals `expected` (hex, any case).
pub fn check_sha256(actual: &str, expected: &str) -> Result<()> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        bail!("The downloaded file does not match the expected SHA-256 (expected {expected}, got {actual}).")
    }
}

/// Copies the adapter executable out of an archive into `dest`. Every other file in the archive
/// (the Claude CLI included) is skipped.
fn extract_adapter(archive: &Path, asset: &Asset, dest: &Path) -> Result<()> {
    let want = asset.exe_name();
    let is_adapter = |name: &str| name.rsplit(['/', '\\']).next() == Some(want);
    let mut out = std::fs::File::create(dest)?;
    let found = match asset.format {
        Format::Zip => {
            let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
            let mut found = false;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)?;
                let named = entry.name().is_ok_and(|n| is_adapter(&n));
                if entry.is_file() && named {
                    std::io::copy(&mut entry, &mut out)?;
                    found = true;
                    break;
                }
            }
            found
        }
        Format::TarGz => {
            let gz = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
            let mut tar = tar::Archive::new(gz);
            let mut found = false;
            for entry in tar.entries()? {
                let mut entry = entry?;
                let path = entry.path()?.to_string_lossy().to_string();
                if entry.header().entry_type().is_file() && is_adapter(&path) {
                    std::io::copy(&mut entry, &mut out)?;
                    found = true;
                    break;
                }
            }
            found
        }
    };
    out.flush()?;
    drop(out);
    if !found {
        let _ = std::fs::remove_file(dest);
        bail!("The archive has no {want} file.");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Checks the archive against the asset's pinned hash, then writes only the adapter to
/// `install_dir(data_root, version)`. Returns the adapter path.
pub fn install_from_archive(
    archive: &Path,
    asset: &Asset,
    data_root: &Path,
    version: &str,
) -> Result<PathBuf> {
    check_sha256(&sha256_file(archive)?, asset.sha256)?;
    let dir = install_dir(data_root, version);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(asset.exe_name());
    let tmp = dir.join(format!("{}.tmp", asset.exe_name()));
    extract_adapter(archive, asset, &tmp)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Removes adapter folders of other versions.
fn prune_old(data_root: &Path) {
    let keep = install_dir(data_root, VERSION);
    let Ok(rd) = std::fs::read_dir(data_root.join("adapters")) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let ours = p
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(&format!("{NAME}-")));
        if ours && p != keep && p.is_dir() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Downloads and installs the pinned adapter for this machine into `data_root`.
/// `progress(received, total)` is called as bytes arrive. Returns the adapter path.
pub async fn download(
    data_root: &Path,
    mut progress: impl FnMut(u64, u64) + Send,
) -> Result<PathBuf> {
    let asset = current().ok_or_else(|| {
        anyhow!(
            "There is no claude-agent-acp build for {} {}.",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let _guard = LOCK.lock().await;
    let dest = install_dir(data_root, VERSION).join(asset.exe_name());
    if dest.is_file() {
        return Ok(dest);
    }
    let work = data_root.join("adapters").join(".download");
    std::fs::create_dir_all(&work)?;
    let archive = work.join(asset.file_name());

    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("FocusIDE/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let url = asset.url();
    let mut res = client
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .with_context(|| format!("Download failed: {url}"))?;
    let total = res.content_length().unwrap_or(asset.size);
    let mut file = std::fs::File::create(&archive)?;
    let mut got = 0u64;
    while let Some(chunk) = res
        .chunk()
        .await
        .with_context(|| format!("Download failed: {url}"))?
    {
        file.write_all(&chunk)?;
        got += chunk.len() as u64;
        progress(got, total);
    }
    file.flush()?;
    drop(file);

    let root = data_root.to_path_buf();
    let a = archive.clone();
    let result =
        tokio::task::spawn_blocking(move || install_from_archive(&a, asset, &root, VERSION))
            .await
            .map_err(|e| anyhow!("{e}"))?;
    let _ = std::fs::remove_dir_all(&work);
    let path = result?;
    prune_old(data_root);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_asset_per_target() {
        let a = asset_for("windows", "x86_64").unwrap();
        assert_eq!(a.format, Format::Zip);
        assert_eq!(a.exe_name(), "claude-agent-acp.exe");
        assert_eq!(
            a.url(),
            "https://github.com/agentclientprotocol/claude-agent-acp/releases/download/v0.106.0/claude-agent-acp-windows-x64.zip"
        );
        assert_eq!(
            asset_for("windows", "aarch64").unwrap().file,
            "windows-arm64.zip"
        );
        assert_eq!(
            asset_for("macos", "aarch64").unwrap().file,
            "darwin-arm64.zip"
        );
        assert_eq!(asset_for("macos", "x86_64").unwrap().file, "darwin-x64.zip");
        let l = asset_for("linux", "x86_64").unwrap();
        assert_eq!(l.file, "linux-x64.tar.gz");
        assert_eq!(l.format, Format::TarGz);
        assert_eq!(l.exe_name(), "claude-agent-acp");
        assert_eq!(
            asset_for("linux", "aarch64").unwrap().file,
            "linux-arm64.tar.gz"
        );
        assert!(asset_for("freebsd", "x86_64").is_none());
        assert!(asset_for("linux", "x86").is_none());
        assert!(current().is_some());
        for a in ASSETS {
            assert_eq!(a.sha256.len(), 64, "{}", a.file);
            assert!(a.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(a.size > 1_000_000);
        }
    }

    #[test]
    fn versions_the_folder() {
        let root = Path::new("data");
        assert_eq!(
            install_dir(root, "0.106.0"),
            root.join("adapters").join("claude-agent-acp-0.106.0")
        );
        assert_ne!(install_dir(root, "0.106.0"), install_dir(root, "0.107.0"));
        let p = adapter_path(root).unwrap();
        assert!(p.starts_with(install_dir(root, VERSION)));
    }

    fn fake(format: Format, sha256: &'static str) -> Asset {
        Asset {
            os: "linux",
            arch: "x86_64",
            file: "test",
            format,
            sha256,
            size: 0,
        }
    }

    fn tar_gz(path: &Path) {
        let f = std::fs::File::create(path).unwrap();
        let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (name, body) in [
            ("pkg/claude", &b"cli"[..]),
            ("pkg/claude-agent-acp", &b"adapter"[..]),
        ] {
            let mut h = tar::Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(0o755);
            h.set_cksum();
            b.append_data(&mut h, name, body).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn rejects_hash_mismatch() {
        assert!(check_sha256("ab", "AB").is_ok());
        assert!(check_sha256("ab", "cd").is_err());
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        tar_gz(&archive);
        let wrong = "0".repeat(64).leak();
        let err = install_from_archive(&archive, &fake(Format::TarGz, wrong), dir.path(), "1.0.0");
        assert!(err.unwrap_err().to_string().contains("SHA-256"));
        assert!(!install_dir(dir.path(), "1.0.0").exists());
    }

    #[test]
    fn keeps_only_the_adapter_from_tar_gz() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        tar_gz(&archive);
        let hash = sha256_file(&archive).unwrap().leak();
        let p = install_from_archive(&archive, &fake(Format::TarGz, hash), dir.path(), "1.0.0")
            .unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"adapter");
        let files: Vec<_> = std::fs::read_dir(install_dir(dir.path(), "1.0.0"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(files, vec!["claude-agent-acp"]);
    }

    #[test]
    fn keeps_only_the_adapter_from_zip() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("claude.exe", o).unwrap();
            z.write_all(b"cli").unwrap();
            z.start_file("claude-agent-acp.exe", o).unwrap();
            z.write_all(b"adapter").unwrap();
            z.finish().unwrap();
        }
        let hash = sha256_file(&archive).unwrap().leak();
        let mut asset = fake(Format::Zip, hash);
        asset.os = "windows";
        let p = install_from_archive(&archive, &asset, dir.path(), "1.0.0").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"adapter");
        assert!(!install_dir(dir.path(), "1.0.0").join("claude.exe").exists());
    }

    /// Downloads the real pinned release for this machine:
    /// `cargo test -p focuside-core -- --ignored real_download --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn real_download() {
        let dir = tempfile::tempdir().unwrap();
        let mut last = 0;
        let p = download(dir.path(), |got, total| {
            let pct = got * 100 / total.max(1);
            if pct >= last + 25 {
                last = pct;
                println!("{got} / {total}");
            }
        })
        .await
        .unwrap();
        let files: Vec<_> = std::fs::read_dir(p.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        println!(
            "{} {:?} {} bytes",
            p.display(),
            files,
            std::fs::metadata(&p).unwrap().len()
        );
        assert_eq!(files, vec![current().unwrap().exe_name()]);
        assert!(!dir.path().join("adapters").join(".download").exists());
    }
}
