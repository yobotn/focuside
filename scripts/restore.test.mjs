// Unit tests for apps/desktop/src/lib/restore.ts: what comes back when FocusIDE opens again.
// Run: node --test scripts/ (Node 22.18 or later runs the TypeScript file directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { loadPanels, loadTabs, loadTurns, placeTurns, tabsOf, turnsOf } from "../apps/desktop/src/lib/restore.ts";

const result = (turnId, files = ["a.txt"]) => ({
  turnId,
  stopReason: "end_turn",
  error: null,
  hasSnapshot: true,
  changes: { files: files.map((path) => ({ path, status: "modified", added: 1, removed: 0 })), generated: [] },
});

/** The state file goes through JSON, as ui_state_save and ui_state_load do. */
const viaJson = (v) => JSON.parse(JSON.stringify(v));

test("tabs round trip: order, active tab, sessions not worth saving left out", () => {
  const sessions = [
    { key: "s2", saved: () => ({ acpId: "pane-s2" }) },
    { key: "s1", saved: () => ({ acpId: "chat-1" }) },
    // An unused "Session N": not saved, so not restored.
    { key: "s3", saved: () => null },
  ];
  const tabs = tabsOf(sessions, "s1");
  assert.deepEqual(tabs, { ids: ["pane-s2", "chat-1"], active: "chat-1" });
  const back = loadTabs(viaJson({ "C:\\p": tabs }));
  assert.deepEqual(back["C:\\p"], tabs);
  // The active tab was not saved: none is active.
  assert.equal(tabsOf(sessions, "s3").active, null);
});

test("tabs from a bad or old state file", () => {
  assert.deepEqual(loadTabs(undefined), {});
  assert.deepEqual(loadTabs("x"), {});
  assert.deepEqual(loadTabs({ a: null, b: { ids: "x" } }), {});
  assert.deepEqual(loadTabs({ a: { ids: ["x", "x", 3, "", "y"], active: "z" } }), { a: { ids: ["x", "y"], active: null } });
});

test("panel visibility round trip", () => {
  const panels = { plan: true, prompts: false, terminal: true };
  assert.deepEqual(loadPanels(viaJson(panels)), panels);
  // State saved before panels were kept: all hidden.
  assert.deepEqual(loadPanels(undefined), { plan: false, prompts: false, terminal: false });
  assert.deepEqual(loadPanels({ plan: "yes" }), { plan: false, prompts: false, terminal: false });
});

test("chat changes cards go back after their messages in the replayed history", () => {
  const before = [
    { t: "user", id: "u1", text: "Fix the bug" },
    { t: "agent", id: "a1", text: "Done." },
    { t: "changes", id: "c1", turnId: "t1", result: result("t1"), undo: null },
    { t: "user", id: "u2", text: "Add a test" },
    { t: "agent", id: "a2", text: "Added." },
    { t: "changes", id: "c2", turnId: "t2", result: result("t2", ["b.txt"]), undo: { outcome: { exact: true } } },
    // No changes: not kept.
    { t: "changes", id: "c3", turnId: "t3", result: result("t3", []), undo: null },
  ];
  const saved = loadTurns(viaJson(turnsOf(before)));
  assert.deepEqual(
    saved.map((x) => [x.turnId, x.prompt, x.text]),
    [
      ["t1", 0, "Fix the bug"],
      ["t2", 1, "Add a test"],
    ],
  );
  assert.deepEqual(saved[1].undo, { outcome: { exact: true } });

  // session/load replays the messages only.
  const replay = [
    { t: "user", id: "r1", text: "Fix the bug" },
    { t: "agent", id: "r2", text: "Done." },
    { t: "user", id: "r3", text: "Add a test" },
    { t: "agent", id: "r4", text: "Added." },
  ];
  let n = 0;
  placeTurns(replay, saved, () => `n${++n}`);
  assert.deepEqual(
    replay.map((e) => (e.t === "changes" ? e.turnId : e.id)),
    ["r1", "r2", "t1", "r3", "r4", "t2"],
  );
});

test("a card whose message moved is found by its text; an unknown one goes last", () => {
  const saved = [
    { turnId: "t1", result: result("t1"), undo: null, prompt: 0, text: "Second" },
    { turnId: "t2", result: result("t2"), undo: null, prompt: 7, text: "Gone" },
  ];
  const replay = [
    { t: "user", id: "r0", text: "Extra" },
    { t: "user", id: "r1", text: "Second" },
    { t: "user", id: "r2", text: "Third" },
  ];
  placeTurns(replay, saved, () => "n");
  assert.deepEqual(
    replay.map((e) => (e.t === "changes" ? e.turnId : e.id)),
    ["r0", "r1", "t1", "r2", "t2"],
  );
});

test("pane turns and bad saved turns", () => {
  const pane = turnsOf([{ t: "changes", id: "c", turnId: "t1", result: result("t1"), undo: null }]);
  assert.equal(pane[0].prompt, -1);
  assert.equal(pane[0].text, undefined);
  assert.deepEqual(loadTurns(null), []);
  assert.deepEqual(loadTurns([null, { turnId: 3 }, { turnId: "x", result: { hasSnapshot: false } }]), []);
  const entries = [];
  placeTurns(entries, loadTurns(viaJson(pane)), () => "n");
  assert.equal(entries.length, 1);
});
