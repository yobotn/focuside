// Builds the focuside-hook helper (crates/hook), which FocusIDE ships as a sidecar and copies
// into the Claude plugin folder it generates in app data.
//   node scripts/build-hook.mjs            debug build into target/debug, next to the dev app
//   node scripts/build-hook.mjs --release  release build, copied to
//                                          apps/desktop/src-tauri/binaries/focuside-hook-<triple>
// The release form runs in the bundle config's beforeBuildCommand, so `npm run build` and the
// release workflow both have the file before tauri-build checks bundle.externalBin.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const release = process.argv.includes("--release");

function hostTriple() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const line = out.split(/\r?\n/).find((l) => l.startsWith("host:"));
  if (!line) throw new Error("rustc -vV printed no host triple");
  return line.slice(5).trim();
}

const host = hostTriple();
// The Tauri CLI sets this for before-build commands; it differs from the host when cross-building.
const triple = process.env.TAURI_ENV_TARGET_TRIPLE || host;
const args = ["build", "-p", "focuside-hook"];
if (release) args.push("--release");
if (triple !== host) args.push("--target", triple);
execFileSync("cargo", args, { cwd: root, stdio: "inherit" });

if (release) {
  const exe = triple.includes("windows") ? ".exe" : "";
  const outDir = triple !== host ? join(root, "target", triple, "release") : join(root, "target", "release");
  const binDir = join(root, "apps/desktop/src-tauri/binaries");
  mkdirSync(binDir, { recursive: true });
  const dest = join(binDir, `focuside-hook-${triple}${exe}`);
  copyFileSync(join(outDir, `focuside-hook${exe}`), dest);
  console.log(`focuside-hook copied to ${dest}`);
}
