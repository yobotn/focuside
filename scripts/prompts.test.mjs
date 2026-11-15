// Unit tests for apps/desktop/src/lib/prompts.ts: prompt states, search, the dot strip with
// many prompts, stepping and "Undo back to this prompt".
// Run: node --test scripts/ (Node 22.18 or later runs the TypeScript file directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  bucketDots,
  dotOf,
  filterPrompts,
  firstLine,
  matchRanges,
  promptState,
  splitMatches,
  stepPrompt,
  undoBackFiles,
  undoBackTurns,
} from "../apps/desktop/src/lib/prompts.ts";

const result = (turnId, files = [], extra = {}) => ({
  turnId,
  stopReason: "end_turn",
  error: null,
  hasSnapshot: true,
  changes: { files: files.map((path) => ({ path, status: "modified", added: 1, removed: 0 })), generated: [], added: files.length, removed: 0 },
  ...extra,
});

const prompt = (id, text, answer = null, more = {}) => ({ id, at: null, prompt: text, answer, turnId: null, result: null, undo: null, ...more });

test("prompt states", () => {
  assert.equal(promptState(prompt("a", "x"), true, true), "working");
  // Not the last prompt, or not busy: no result is a done prompt without changes.
  assert.equal(promptState(prompt("a", "x"), false, true), "done");
  assert.equal(promptState(prompt("a", "x"), true, false), "done");
  assert.equal(promptState(prompt("a", "x", "ok", { turnId: "t", result: result("t", ["a.txt"]) }), false, false), "changed");
  assert.equal(promptState(prompt("a", "x", null, { turnId: "t", result: result("t", [], { error: "boom" }) }), false, false), "failed");
  // Panes: a StopFailure with an unknown type is "" and still a failure.
  assert.equal(promptState(prompt("a", "x", null, { result: result("t", [], { failure: "" }) }), false, false), "failed");
  assert.equal(promptState(prompt("a", "x", null, { result: result("t", [], { failure: null }) }), false, false), "done");
  const undo = { outcome: { restored: [], kept: [], failed: [], exact: true } };
  assert.equal(promptState(prompt("a", "x", null, { turnId: "t", result: result("t", ["a"]), undo }), false, false), "undone");
  assert.equal(promptState(prompt("a", "x", null, { untracked: "Session 2" }), false, false), "untracked");
  assert.equal(promptState(prompt("a", "x", null, { untracked: "" }), false, false), "untracked");
});

test("prompts stored before this change still work: no untracked field, no changes", () => {
  const old = { id: "p1", at: 1, prompt: "Fix it", answer: "Done.", turnId: "t1", result: { turnId: "t1", stopReason: "end_turn", error: null, hasSnapshot: true }, undo: null };
  assert.equal(promptState(old, false, false), "done");
  assert.deepEqual(filterPrompts([old], "done"), [0]);
  assert.deepEqual(undoBackTurns([prompt("p0", "a"), old], 0), []);
});

test("first line", () => {
  assert.equal(firstLine("  \n hello \nworld"), "hello");
  assert.equal(firstLine(""), "");
  assert.equal(firstLine("x".repeat(100), 10), "xxxxxxx...");
});

test("search: filter and match ranges without case", () => {
  const list = [prompt("1", "Add a Button"), prompt("2", "Other", "The button is blue"), prompt("3", "Nothing", null)];
  assert.deepEqual(filterPrompts(list, "button"), [0, 1]);
  assert.deepEqual(filterPrompts(list, "  "), [0, 1, 2]);
  assert.deepEqual(filterPrompts(list, "zzz"), []);
  assert.deepEqual(matchRanges("aAa", "a"), [[0, 1], [1, 2], [2, 3]]);
  assert.deepEqual(matchRanges("aaaa", "aa"), [[0, 2], [2, 4]]);
  assert.deepEqual(matchRanges("abc", ""), []);
  assert.deepEqual(splitMatches("Fix the Bug now", "bug"), [
    { text: "Fix the ", hit: false },
    { text: "Bug", hit: true },
    { text: " now", hit: false },
  ]);
  assert.deepEqual(splitMatches("abc", "x"), [{ text: "abc", hit: false }]);
});

test("stepping from prompt to prompt", () => {
  const all = [0, 1, 2, 3, 4];
  assert.equal(stepPrompt(all, 2, "next"), 3);
  assert.equal(stepPrompt(all, 2, "prev"), 1);
  assert.equal(stepPrompt(all, 4, "next"), 4);
  assert.equal(stepPrompt(all, 0, "prev"), 0);
  assert.equal(stepPrompt(all, 2, "first"), 0);
  assert.equal(stepPrompt(all, 2, "last"), 4);
  assert.equal(stepPrompt(all, -1, "next"), 0);
  assert.equal(stepPrompt(all, -1, "prev"), 4);
  // The search hides the current prompt: go to the nearest match in that direction.
  const some = [1, 4, 7];
  assert.equal(stepPrompt(some, 5, "next"), 7);
  assert.equal(stepPrompt(some, 5, "prev"), 4);
  assert.equal(stepPrompt(some, 9, "next"), 7);
  assert.equal(stepPrompt(some, 0, "prev"), 1);
  assert.equal(stepPrompt([], 3, "next"), -1);
});

test("dots: one per prompt when they fit, buckets when they do not", () => {
  const few = bucketDots(["done", "changed", "working"], 10);
  assert.deepEqual(few, [
    { start: 0, end: 0, state: "done" },
    { start: 1, end: 1, state: "changed" },
    { start: 2, end: 2, state: "working" },
  ]);
  const states = Array.from({ length: 205 }, (_, i) => (i === 7 ? "failed" : i === 6 ? "changed" : "done"));
  const dots = bucketDots(states, 50);
  assert.ok(dots.length <= 50);
  assert.equal(dots[0].start, 0);
  assert.equal(dots[dots.length - 1].end, 204);
  // Every prompt is in exactly one dot, in order.
  for (let i = 1; i < dots.length; i++) assert.equal(dots[i].start, dots[i - 1].end + 1);
  // The heaviest state wins: failed over changed over done.
  assert.equal(dots[dotOf(dots, 7)].state, "failed");
  assert.equal(dots[dotOf(dots, 100)].state, "done");
  assert.equal(dotOf(dots, 205), -1);
  assert.deepEqual(bucketDots([], 10), []);
  // No room still gives dots (one bucket per prompt at room 1 is all prompts in one dot).
  assert.equal(bucketDots(states, 0).length, 1);
});

test("undo back to a prompt: later tracked turns with changes, newest first", () => {
  const undo = { outcome: { restored: [], kept: [], failed: [], exact: true } };
  const list = [
    prompt("p0", "a", null, { turnId: "t0", result: result("t0", ["a.txt"]) }),
    prompt("p1", "b", null, { turnId: "t1", result: result("t1", ["b.txt", "a.txt"]) }),
    prompt("p2", "c", null, { turnId: "t2", result: result("t2", []) }),
    prompt("p3", "d", null, { turnId: "t3", result: result("t3", ["c.txt"]), undo }),
    prompt("p4", "e", null, { untracked: "Session 2" }),
    prompt("p5", "f", null, { turnId: "t5", result: result("t5", ["d.txt", "b.txt"]) }),
  ];
  const turns = undoBackTurns(list, 0);
  assert.deepEqual(turns.map((p) => p.id), ["p5", "p1"]);
  assert.deepEqual(undoBackFiles(turns), ["a.txt", "b.txt", "d.txt"]);
  assert.deepEqual(undoBackTurns(list, 5), []);
  assert.deepEqual(undoBackTurns(list, 1).map((p) => p.id), ["p5"]);
});
