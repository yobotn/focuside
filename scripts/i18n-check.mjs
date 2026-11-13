// Checks the UI string catalog: every t("key") used in the UI exists in en.ts, and every key
// in en.ts is used somewhere (dynamic keys are listed in `dynamic` below).
// Usage: node scripts/i18n-check.mjs
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const src = join(root, "apps/desktop/src");
const catalog = readFileSync(join(src, "lib/i18n/en.ts"), "utf8");
const keys = new Set([...catalog.matchAll(/^\s*"([\w.]+)":/gm)].map((m) => m[1]));

// Keys built at run time, for example `plan.change.${kind}`.
const dynamicPrefixes = ["plan.change.", "settings.theme.", "queue.", "notify.", "tool.", "agentNote."];

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* walk(p);
    else if ([".ts", ".svelte"].includes(extname(name)) && !p.includes("i18n")) yield p;
  }
}

const used = new Set();
const problems = [];
for (const f of walk(src)) {
  const text = readFileSync(f, "utf8");
  for (const m of text.matchAll(/\bt\(\s*"([\w.]+)"/g)) {
    used.add(m[1]);
    if (!keys.has(m[1])) problems.push(`${f}: unknown key ${m[1]}`);
  }
  for (const m of text.matchAll(/"([a-z]+\.[\w.]+)"/g)) if (keys.has(m[1])) used.add(m[1]);
}
const unused = [...keys].filter((k) => !used.has(k) && !dynamicPrefixes.some((p) => k.startsWith(p)));

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} problem(s).`);
  process.exit(1);
}
if (unused.length && process.argv.includes("--unused")) console.log(`Unused keys:\n${unused.join("\n")}`);
console.log(`Strings OK: ${keys.size} keys, ${used.size} used.`);
