// Unit tests for apps/desktop/src/lib/review.ts and lib/keys.ts: diff blocks, reviewed files,
// session tab state, notifications, recent commands and the shortcut table.
// Run: node --test scripts/ (Node 22.18 or later runs the TypeScript files directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  diffBlocks,
  isReviewed,
  loadNotify,
  pairRows,
  pushRecent,
  reviewItems,
  sessionFiles,
  setReviewed,
  shouldNotify,
  splitDuration,
  splitLines,
  stepHunk,
  tabState,
  unreviewedCount,
  withRecent,
} from "../apps/desktop/src/lib/review.ts";
import { SHORTCUTS, keysOf } from "../apps/desktop/src/lib/keys.ts";

const result = (turnId, files) => ({
  turnId,
  stopReason: "end_turn",
  error: null,
  hasSnapshot: true,
  changes: { files: files.map((path) => ({ path, status: "edit", added: 1, removed: 0, binary: false })), generated: [], added: 1, removed: 0 },
});
const changes = (turnId, files, extra = {}) => ({ t: "changes", id: turnId, turnId, result: result(turnId, files), undo: null, ...extra });

test("lines are counted like Rust's split_inclusive", () => {
  assert.deepEqual(splitLines("a\nb\n"), ["a", "b"]);
  assert.deepEqual(splitLines("a\r\nb"), ["a", "b"]);
  assert.deepEqual(splitLines(""), []);
  assert.deepEqual(splitLines(null), []);
  assert.deepEqual(splitLines("\n"), [""]);
});

test("diff blocks: context around hunks, gaps between, expanding a gap", () => {
  const before = Array.from({ length: 20 }, (_, i) => `l${i + 1}`).join("\n") + "\n";
  const after = before.replace("l2\n", "L2\n").replace("l18\n", "L18\nnew\n");
  const hunks = [
    { oldStart: 1, oldLen: 1, newStart: 1, newLen: 1, state: "agent" },
    { oldStart: 17, oldLen: 1, newStart: 17, newLen: 2, state: "undone" },
  ];
  const blocks = diffBlocks(before, after, hunks);
  assert.deepEqual(
    blocks.map((b) => b.kind),
    ["context", "hunk", "context", "gap", "context", "hunk", "context"],
  );
  // One line above the first hunk; three after it, three before the next.
  assert.deepEqual(blocks[0].lines, [{ old: 1, new: 1, text: "l1" }]);
  assert.deepEqual(blocks[1].del, [{ old: 2, new: null, text: "l2" }]);
  assert.deepEqual(blocks[1].add, [{ old: null, new: 2, text: "L2" }]);
  assert.deepEqual(blocks[3], { kind: "gap", at: 5, count: 9 });
  assert.equal(blocks[5].state, "undone");
  assert.deepEqual(blocks[5].add.map((l) => l.text), ["L18", "new"]);
  // After the second hunk the new line numbers are one higher.
  assert.deepEqual(blocks[6].lines[0], { old: 19, new: 20, text: "l19" });
  // An expanded gap shows its lines.
  const open = diffBlocks(before, after, hunks, new Set([5]));
  assert.ok(!open.some((b) => b.kind === "gap"));
  assert.equal(open.filter((b) => b.kind === "context").reduce((n, b) => n + b.lines.length, 0), 18);
});

test("diff blocks of a new file and side by side rows", () => {
  const blocks = diffBlocks(null, "a\nb\n", [{ oldStart: 0, oldLen: 0, newStart: 0, newLen: 2, state: "agent" }]);
  assert.equal(blocks.length, 1);
  assert.deepEqual(blocks[0].add.map((l) => l.new), [1, 2]);
  const rows = pairRows([{ old: 1, new: null, text: "x" }], blocks[0].add);
  assert.equal(rows.length, 2);
  assert.equal(rows[0].left.text, "x");
  assert.equal(rows[1].left, null);
});

test("reviewed files, unreviewed count and session files", () => {
  const entries = [
    { t: "user", id: "u1", text: "hi" },
    // A turn from before reviews: counts as reviewed.
    changes("t0", ["old.txt"]),
    changes("t1", ["a.txt", "b.txt"], { reviewed: ["a.txt"] }),
    changes("t2", ["c.txt", "a.txt"], { reviewed: [] }),
    changes("t3", ["d.txt"], { reviewed: [], undo: { outcome: { restored: [], kept: [], failed: [], exact: true } } }),
  ];
  assert.equal(isReviewed(entries[1], "old.txt"), true);
  assert.equal(isReviewed(entries[2], "b.txt"), false);
  assert.equal(unreviewedCount(entries), 3);
  assert.deepEqual(sessionFiles(entries), ["old.txt", "a.txt", "b.txt", "c.txt"]);
  // One turn, also when undone; the session leaves undone turns out.
  assert.deepEqual(reviewItems(entries, "t3").map((x) => x.file.path), ["d.txt"]);
  assert.deepEqual(
    reviewItems(entries, null).map((x) => `${x.turnId}:${x.file.path}`),
    ["t0:old.txt", "t1:a.txt", "t1:b.txt", "t2:c.txt", "t2:a.txt"],
  );
  assert.deepEqual(setReviewed(undefined, "x", true), ["x"]);
  assert.deepEqual(setReviewed(["x"], "x", true), ["x"]);
  assert.deepEqual(setReviewed(["x", "y"], "x", false), ["y"]);
});

test("session tab state", () => {
  const base = { dormant: false, busy: false, waiting: null, status: "ready", unreviewed: 0 };
  assert.equal(tabState(base), "idle");
  assert.equal(tabState({ ...base, dormant: true, busy: true }), "dormant");
  assert.equal(tabState({ ...base, busy: true, waiting: "plan", unreviewed: 2 }), "working");
  assert.equal(tabState({ ...base, waiting: "plan" }), "waiting");
  assert.equal(tabState({ ...base, waiting: "permission", unreviewed: 1 }), "waiting");
  assert.equal(tabState({ ...base, waiting: "failed", unreviewed: 1 }), "failed");
  assert.equal(tabState({ ...base, status: "failed" }), "failed");
  assert.equal(tabState({ ...base, status: "starting" }), "starting");
  assert.equal(tabState({ ...base, waiting: "done", unreviewed: 1 }), "review");
  assert.equal(tabState({ ...base, unreviewed: 1 }), "review");
  assert.equal(tabState({ ...base, waiting: "done" }), "done");
});

test("durations", () => {
  assert.deepEqual(splitDuration(0), { h: 0, m: 0, s: 0 });
  assert.deepEqual(splitDuration(65_400), { h: 0, m: 1, s: 5 });
  assert.deepEqual(splitDuration(3_725_000), { h: 1, m: 2, s: 5 });
  assert.deepEqual(splitDuration(-5), { h: 0, m: 0, s: 0 });
});

test("next and previous change stop at the ends", () => {
  assert.equal(stepHunk(-1, 0, 1), -1);
  assert.equal(stepHunk(-1, 3, 1), 0);
  assert.equal(stepHunk(-1, 3, -1), 2);
  assert.equal(stepHunk(2, 3, 1), 2);
  assert.equal(stepHunk(0, 3, -1), 0);
  assert.equal(stepHunk(1, 3, 1), 2);
});

test("notifications: never, in the background, always", () => {
  assert.equal(shouldNotify("never", false, false), false);
  assert.equal(shouldNotify("background", false, false), true);
  assert.equal(shouldNotify("background", false, true), true);
  assert.equal(shouldNotify("background", true, false), false);
  assert.equal(shouldNotify("always", true, false), true);
  assert.equal(shouldNotify("always", true, true), false);
  // The old checkbox: off became "never", anything else the default.
  assert.equal(loadNotify(undefined, false), "never");
  assert.equal(loadNotify(undefined, true), "background");
  assert.equal(loadNotify("always", false), "always");
  assert.equal(loadNotify("loud", undefined), "background");
});

test("recent commands", () => {
  const items = [{ id: "a" }, { id: "b" }, { id: "c" }];
  const { recent, rest } = withRecent(items, ["c", "gone", "a"]);
  assert.deepEqual(recent.map((x) => x.id), ["c", "a"]);
  assert.deepEqual(rest.map((x) => x.id), ["b"]);
  assert.deepEqual(pushRecent(["a", "b"], "b"), ["b", "a"]);
  assert.deepEqual(pushRecent(["a", "b", "c", "d", "e"], "f"), ["f", "a", "b", "c", "d"]);
});

test("shortcut table: ids once, no key used twice in the same place", () => {
  const ids = SHORTCUTS.map((s) => s.id);
  assert.equal(new Set(ids).size, ids.length);
  const seen = new Map();
  for (const s of SHORTCUTS) {
    for (const k of s.keys) {
      const at = `${s.where ?? "app"}:${k.toLowerCase()}`;
      assert.ok(!seen.has(at), `${k} is used by ${seen.get(at)} and ${s.id}`);
      seen.set(at, s.id);
    }
  }
  assert.deepEqual(keysOf("palette"), ["Mod+K", "Mod+Shift+P"]);
  // Keys that work only in one place are not shown in the palette.
  assert.deepEqual(keysOf("undo"), []);
  assert.deepEqual(keysOf("nothing"), []);
});
