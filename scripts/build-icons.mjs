// Builds the file icon set from Symbols by Miguel Solorio (MIT): downloads the pinned release,
// copies only the SVGs the mapping uses into apps/desktop/src/lib/icons/generated, and writes
// map.json. Fails when the bundle grows over its size budget.
// Usage: node scripts/build-icons.mjs [--force]
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync, copyFileSync } from "node:fs";
import { join } from "node:path";

const VERSION = "0.0.26";
const BUDGET_BYTES = 512 * 1024;
// A single icon over this size (some are traced bitmaps) is dropped: the file falls back to the
// generic icon.
const MAX_ICON_BYTES = 12 * 1024;

/** Small, safe SVG minification: no XML declaration or comments, short numbers, no extra space. */
function minify(svg) {
  return svg
    .replace(/<\?xml[^>]*>/g, "")
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/(\d+\.\d{2})\d+/g, "$1")
    .replace(/\s+/g, " ")
    .replace(/>\s+</g, "><")
    .trim();
}

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const out = join(root, "apps/desktop/src/lib/icons/generated");
const cache = join(root, "node_modules/.cache/symbols-" + VERSION);

if (existsSync(join(out, "map.json")) && !process.argv.includes("--force")) {
  console.log("File icons present.");
  process.exit(0);
}

// VS Code maps extensions to languages, then languages to icons. These are the languages the
// theme maps by id, with the extensions that belong to them.
const languageExtensions = {
  rust: ["rs"],
  typescript: ["ts", "mts", "cts"],
  typescriptreact: ["tsx"],
  javascript: ["js", "mjs", "cjs"],
  javascriptreact: ["jsx"],
  json: ["json", "jsonc", "json5"],
  markdown: ["md", "markdown", "mdx"],
  toml: ["toml"],
  yaml: ["yml", "yaml"],
  python: ["py", "pyi", "pyw"],
  go: ["go"],
  java: ["java"],
  kotlin: ["kt", "kts"],
  swift: ["swift"],
  c: ["c", "h"],
  cpp: ["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
  csharp: ["cs"],
  ruby: ["rb"],
  php: ["php"],
  html: ["html", "htm"],
  css: ["css"],
  scss: ["scss", "sass"],
  less: ["less"],
  shellscript: ["sh", "bash", "zsh", "fish"],
  powershell: ["ps1", "psm1", "psd1"],
  bat: ["bat", "cmd"],
  sql: ["sql"],
  xml: ["xml", "xsd", "xsl"],
  svelte: ["svelte"],
  vue: ["vue"],
  dockerfile: ["dockerfile"],
  lua: ["lua"],
  dart: ["dart"],
  elixir: ["ex", "exs"],
  haskell: ["hs"],
  scala: ["scala"],
  r: ["r"],
  zig: ["zig"],
  ini: ["ini", "cfg"],
  plaintext: ["txt"],
  makefile: ["mk"],
  graphql: ["graphql", "gql"],
  astro: ["astro"],
  prisma: ["prisma"],
  terraform: ["tf", "tfvars"],
  nix: ["nix"],
  ocaml: ["ml", "mli"],
  clojure: ["clj", "cljs", "edn"],
  perl: ["pl", "pm"],
  julia: ["jl"],
  "objective-c": ["m"],
  fsharp: ["fs", "fsx"],
  vb: ["vb"],
  coffeescript: ["coffee"],
  handlebars: ["hbs"],
  pug: ["pug"],
  latex: ["tex"],
  diff: ["diff", "patch"],
  log: ["log"],
  properties: ["properties"],
  csv: ["csv"],
};

if (!existsSync(join(cache, "src/symbol-icon-theme.json"))) {
  mkdirSync(cache, { recursive: true });
  const url = `https://github.com/miguelsolorio/vscode-symbols/archive/refs/tags/${VERSION}.tar.gz`;
  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) {
    console.error(`Download failed: ${res.status}`);
    process.exit(1);
  }
  const tgz = join(cache, "symbols.tgz");
  writeFileSync(tgz, Buffer.from(await res.arrayBuffer()));
  const tar = process.platform === "win32" ? join(process.env.SystemRoot ?? "C:/Windows", "System32", "tar.exe") : "tar";
  execFileSync(tar, ["-xzf", tgz, "-C", cache, "--strip-components=1"], { stdio: "inherit" });
}

const theme = JSON.parse(readFileSync(join(cache, "src/symbol-icon-theme.json"), "utf8"));
const defs = theme.iconDefinitions;
const map = {
  file: theme.file,
  folder: theme.folder,
  folderOpen: theme.folderExpanded ?? theme.folder,
  extensions: { ...theme.fileExtensions },
  names: { ...theme.fileNames },
  folders: { ...theme.folderNames },
  foldersOpen: { ...(theme.folderNamesExpanded ?? {}) },
};
for (const [lang, exts] of Object.entries(languageExtensions)) {
  const icon = theme.languageIds?.[lang];
  if (!icon) continue;
  for (const e of exts) if (!map.extensions[e]) map.extensions[e] = icon;
}
// File names the theme leaves to the extension, where a better icon exists.
const nameOverrides = { "cargo.toml": "rust", "cargo.lock": "rust", "rust-toolchain.toml": "rust" };
for (const [n, icon] of Object.entries(nameOverrides)) if (!map.names[n] && defs[icon]) map.names[n] = icon;

// Lower-case keys: lookups are case-insensitive.
for (const k of ["extensions", "names", "folders", "foldersOpen"]) {
  map[k] = Object.fromEntries(Object.entries(map[k]).map(([a, b]) => [a.toLowerCase(), b]));
}

const used = new Set([map.file, map.folder, map.folderOpen]);
for (const k of ["extensions", "names", "folders", "foldersOpen"]) for (const v of Object.values(map[k])) used.add(v);

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
let total = 0;
const missing = [];
for (const id of used) {
  const def = defs[id];
  if (!def?.iconPath) {
    missing.push(id);
    continue;
  }
  const src = join(cache, "src", def.iconPath);
  if (!existsSync(src)) {
    missing.push(id);
    continue;
  }
  const svg = minify(readFileSync(src, "utf8"));
  if (svg.length > MAX_ICON_BYTES) {
    missing.push(id);
    continue;
  }
  writeFileSync(join(out, `${id}.svg`), svg);
  total += svg.length;
}
// Drop mappings whose icon is missing.
for (const k of ["extensions", "names", "folders", "foldersOpen"]) {
  for (const [a, b] of Object.entries(map[k])) if (missing.includes(b)) delete map[k][a];
}
writeFileSync(join(out, "map.json"), JSON.stringify(map));
copyFileSync(join(cache, "LICENSE"), join(out, "LICENSE"));

const count = readdirSync(out).filter((f) => f.endsWith(".svg")).length;
console.log(`File icons: ${count} SVGs, ${(total / 1024).toFixed(0)} KB (budget ${BUDGET_BYTES / 1024} KB).`);
if (total > BUDGET_BYTES) {
  console.error("The file icon bundle is over its size budget.");
  process.exit(1);
}
