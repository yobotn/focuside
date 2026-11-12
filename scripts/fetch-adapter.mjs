// Dev setup: downloads the pinned claude-agent-acp release for this OS and puts the executable in
// apps/desktop/src-tauri/binaries/claude-agent-acp-<target triple>[.exe], where `npm run dev` finds it.
// Installed builds do not bundle it: the app downloads the same release on first use
// (crates/core/src/adapter_download.rs). Keep the version and hashes in sync with that file.
// Usage: node scripts/fetch-adapter.mjs
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, createWriteStream, existsSync, mkdirSync, readdirSync, renameSync, rmSync, statSync, chmodSync } from "node:fs";
import { join } from "node:path";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";

// When you change the version, update the hashes here, crates/core/src/adapter_download.rs and
// THIRD_PARTY_NOTICES.md too.
export const ADAPTER_VERSION = "0.106.0";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const outDir = join(root, "apps/desktop/src-tauri/binaries");

const targets = {
  "win32-x64": { asset: "windows-x64.zip", sha256: "5de09fe80ac78260eb56964b92e698bd4c467092f0c88e042438a8e05974903c", triple: "x86_64-pc-windows-msvc" },
  "win32-arm64": { asset: "windows-arm64.zip", sha256: "ffc36b8e00b23942f8214e1f70606a77c1c4d90ef3a366422a093f728e59454c", triple: "aarch64-pc-windows-msvc" },
  "darwin-arm64": { asset: "darwin-arm64.zip", sha256: "446c5ff8303c1fb0caaa5b047566ce4f88c5a70d188ed798d236cfc68f875f9e", triple: "aarch64-apple-darwin" },
  "darwin-x64": { asset: "darwin-x64.zip", sha256: "a6fbfc9814e176615d68fd58dda1887ae2b9ff00a18c9e8b9d75352ce7bd0fba", triple: "x86_64-apple-darwin" },
  "linux-x64": { asset: "linux-x64.tar.gz", sha256: "7d27db341f2a09bd64e54465de8409c624c92827d19f2065f32c599377727c6e", triple: "x86_64-unknown-linux-gnu" },
  "linux-arm64": { asset: "linux-arm64.tar.gz", sha256: "04390d95545916b21df886e92ff24538f7d71a86f657556704747f07de18422d", triple: "aarch64-unknown-linux-gnu" },
};

const key = `${process.platform}-${process.arch}`;
const target = targets[key];
if (!target) {
  console.error(`No claude-agent-acp build for ${key}.`);
  process.exit(1);
}
const ext = process.platform === "win32" ? ".exe" : "";
const dest = join(outDir, `claude-agent-acp-${target.triple}${ext}`);
if (existsSync(dest) && !process.argv.includes("--force")) {
  console.log(`Already present: ${dest}`);
  process.exit(0);
}

mkdirSync(outDir, { recursive: true });
const name = `claude-agent-acp-${target.asset}`;
const url = `https://github.com/agentclientprotocol/claude-agent-acp/releases/download/v${ADAPTER_VERSION}/${name}`;
const work = join(outDir, ".download");
rmSync(work, { recursive: true, force: true });
mkdirSync(work, { recursive: true });
const archive = join(work, name);

console.log(`Downloading ${url}`);
const res = await fetch(url);
if (!res.ok) {
  console.error(`Download failed: ${res.status} ${res.statusText}`);
  process.exit(1);
}
await pipeline(Readable.fromWeb(res.body), createWriteStream(archive));
const hash = createHash("sha256");
await pipeline(createReadStream(archive), hash);
const digest = hash.digest("hex");
if (digest !== target.sha256) {
  rmSync(work, { recursive: true, force: true });
  console.error(`SHA-256 mismatch: expected ${target.sha256}, got ${digest}.`);
  process.exit(1);
}

// tar handles zip on Windows 10+ (bsdtar) and tar.gz everywhere. On Windows use the system
// bsdtar: GNU tar from Git Bash reads "C:" as a remote host.
const tar = process.platform === "win32" ? join(process.env.SystemRoot ?? "C:/Windows", "System32", "tar.exe") : "tar";
execFileSync(tar, ["-xf", archive, "-C", work], { stdio: "inherit" });

function* walk(dir) {
  for (const n of readdirSync(dir)) {
    const p = join(dir, n);
    if (statSync(p).isDirectory()) yield* walk(p);
    else yield p;
  }
}
const files = [...walk(work)];
const exe = files.find((f) => f.endsWith(`claude-agent-acp${ext}`));
if (!exe) {
  console.error("The archive has no claude-agent-acp executable. Files:\n" + files.join("\n"));
  process.exit(1);
}
renameSync(exe, dest);
if (!ext) chmodSync(dest, 0o755);
// The release ships its own Claude CLI next to the adapter. FocusIDE does not use it: it sets
// CLAUDE_CODE_EXECUTABLE to the user's own install. Keep only the adapter.
rmSync(work, { recursive: true, force: true });
console.log(`Saved ${dest}`);
