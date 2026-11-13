// Compares FocusIDE's agent launch commands (AGENTS in crates/core/src/agent.rs) with the ACP
// registry. Run it once a month: vendor CLIs update on their own and flags change.
// Usage: node scripts/check-agents.mjs [--strict]
// It only reports. It exits 1 only with --strict, when a command differs in a way that is not
// listed in `known` below, or when the registry cannot be read. It installs nothing.
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REGISTRY = "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";
const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const strict = process.argv.includes("--strict");

// FocusIDE agent id -> registry id.
const registryId = {
  claude: "claude-acp",
  codex: "codex-acp",
  gemini: "gemini",
  opencode: "opencode",
  qwen: "qwen-code",
  goose: "goose",
  antigravity: "antigravity-acp",
  copilot: "github-copilot-cli",
  cursor: "cursor",
  kiro: "kiro",
  cline: "cline",
  kimi: "kimi-code",
  vibe: "mistral-vibe",
};

// Differences FocusIDE keeps on purpose. Each one is explained in agent.rs.
const known = {
  claude: "FocusIDE starts the adapter it downloads (pinned in scripts/fetch-adapter.mjs), with --hide-claude-auth.",
  antigravity: "The registry lists the agy_acp_server from the IDE extensions; FocusIDE starts `agy acp` (Antigravity CLI docs).",
  kiro: "The registry adds --agent-engine=v3, which overrides the user's saved setting.",
};

/** AGENTS entries from agent.rs: id, program, args. */
function ourAgents() {
  const src = readFileSync(join(root, "crates/core/src/agent.rs"), "utf8");
  const table = src.slice(src.indexOf("pub const AGENTS"));
  const out = [];
  for (const m of table.matchAll(/AgentSpec \{([\s\S]*?)\n    \},/g)) {
    const body = m[1].replace(/^\s*\/\/.*$/gm, "");
    const str = (k) => body.match(new RegExp(`\\b${k}: "([^"]*)"`))?.[1];
    const args = [...(body.match(/\bargs: &\[([^\]]*)\]/)?.[1] ?? "").matchAll(/"([^"]*)"/g)].map((a) => a[1]);
    out.push({ id: str("id"), program: str("program"), args });
  }
  return out;
}

/** The registry's launch command: program name (no folder, no extension) and args. */
function theirCommand(entry) {
  const d = entry.distribution ?? {};
  if (d.npx) return { how: `npx ${d.npx.package}`, program: null, args: d.npx.args ?? [] };
  if (d.uvx) return { how: `uvx ${d.uvx.package}`, program: null, args: d.uvx.args ?? [] };
  if (d.binary) {
    const platforms = Object.keys(d.binary);
    const key = platforms.includes("windows-x86_64") ? "windows-x86_64" : platforms[0];
    const b = d.binary[key];
    const program = b.cmd
      .split(/[\\/]/)
      .pop()
      .replace(/\.(exe|cmd|par)$/i, "");
    const args = (b.args ?? []).filter((a) => a !== "--uid=");
    const note = platforms.some((p) => p.startsWith("windows")) ? "" : ", no Windows build";
    return { how: `binary (${key}${note})`, program, args };
  }
  return null;
}

let registry;
try {
  const res = await fetch(REGISTRY);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  registry = await res.json();
} catch (e) {
  console.error(`Could not read the ACP registry: ${e.message}`);
  process.exitCode = strict ? 1 : 0;
}

const byId = new Map((registry?.agents ?? []).map((a) => [a.id, a]));
let unexpected = 0;
for (const a of registry ? ourAgents() : []) {
  const entry = byId.get(registryId[a.id]);
  const ours = [a.program, ...a.args].join(" ");
  if (!entry) {
    console.log(`?  ${a.id}: not in the registry (ours: ${ours})`);
    if (!known[a.id]) unexpected++;
    continue;
  }
  const c = theirCommand(entry);
  const same =
    a.id !== "claude" &&
    c &&
    (c.program === null || c.program === a.program) &&
    c.args.length === a.args.length &&
    c.args.every((x, i) => x === a.args[i]);
  const theirs = c ? `${c.program ?? "<package bin>"} ${c.args.join(" ")}`.trim() : "no command";
  const line = `${a.id}: ours "${ours}", registry ${entry.version} via ${c?.how ?? "?"} "${theirs}"`;
  if (same) console.log(`ok ${line}`);
  else if (known[a.id]) console.log(`-- ${line}\n   Known: ${known[a.id]}`);
  else {
    console.log(`!! ${line}`);
    unexpected++;
  }
}
if (registry)
  console.log(unexpected ? `\n${unexpected} command(s) differ from the registry. Check the vendor docs.` : "\nNo unexpected differences.");
// exitCode, not process.exit: exiting right after fetch trips a libuv assertion on Windows.
if (registry) process.exitCode = strict && unexpected ? 1 : 0;
