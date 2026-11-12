// Fails if the installer would carry the Claude CLI or the claude-agent-acp adapter. FocusIDE
// downloads the adapter on first use (crates/core/src/adapter_download.rs) and runs the user's
// own `claude`, so neither may be bundled.
// Checks every bundle.externalBin and bundle.resources entry in apps/desktop/src-tauri/tauri*.conf.json
// (resource folders are walked). Folders given as arguments are walked too, for example the
// build output after `tauri build`: node scripts/check-bundle.mjs target/release
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const tauriDir = join(root, "apps/desktop/src-tauri");

/** True for a file name that must not be in the bundle. */
export function banned(name) {
  const n = name.toLowerCase();
  return n === "claude" || n === "claude.exe" || n.startsWith("claude-agent-acp");
}

// Rust build folders that never go into a bundle.
const skipDirs = new Set(["deps", "build", ".fingerprint", "incremental", "examples", "node_modules", ".git"]);

function* walk(p) {
  if (!existsSync(p)) return;
  if (!statSync(p).isDirectory()) {
    yield p;
    return;
  }
  for (const n of readdirSync(p)) {
    if (skipDirs.has(n)) continue;
    yield* walk(join(p, n));
  }
}

const problems = [];
const configs = readdirSync(tauriDir).filter((n) => /^tauri.*\.conf\.json$/.test(n));
for (const file of configs) {
  const bundle = JSON.parse(readFileSync(join(tauriDir, file), "utf8")).bundle ?? {};
  for (const bin of bundle.externalBin ?? []) {
    if (banned(basename(bin))) problems.push(`${file}: externalBin ${bin}`);
  }
  const res = bundle.resources ?? [];
  const pairs = Array.isArray(res) ? res.map((s) => [s, s]) : Object.entries(res);
  for (const [src, dest] of pairs) {
    if (banned(basename(dest))) problems.push(`${file}: resource target ${dest}`);
    // For a glob, walk the folder before the first glob part. That can only report too much.
    const parts = src.split(/[/\\]/);
    const firstGlob = parts.findIndex((s) => /[*?[{]/.test(s));
    const base = firstGlob < 0 ? src : parts.slice(0, firstGlob).join("/") || ".";
    for (const f of walk(resolve(tauriDir, base))) {
      if (banned(basename(f))) problems.push(`${file}: resource ${src} contains ${f}`);
    }
  }
}

for (const dir of process.argv.slice(2)) {
  for (const f of walk(resolve(root, dir))) {
    if (banned(basename(f))) problems.push(`build output: ${f}`);
  }
}

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} problem(s). The installer must not include claude, claude.exe or claude-agent-acp.`);
  process.exit(1);
}
console.log(`Bundle check passed: ${configs.length} config file(s)${process.argv.length > 2 ? ", build output" : ""}.`);
