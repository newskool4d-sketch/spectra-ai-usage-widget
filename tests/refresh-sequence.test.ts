import { test } from "node:test";
import assert from "node:assert/strict";
import { createRefreshSequencer } from "../src/data/refresh-sequence.ts";

test("only the latest ticket per key is current", () => {
  const sequencer = createRefreshSequencer<"codex" | "claude">();
  const first = sequencer.begin("codex");
  const second = sequencer.begin("codex");
  assert.equal(sequencer.isCurrent("codex", first), false);
  assert.equal(sequencer.isCurrent("codex", second), true);
});

test("keys are tracked independently", () => {
  const sequencer = createRefreshSequencer<"codex" | "claude">();
  const codex = sequencer.begin("codex");
  sequencer.begin("claude");
  assert.equal(sequencer.isCurrent("codex", codex), true);
});

test("unknown ticket is never current", () => {
  const sequencer = createRefreshSequencer<"codex">();
  assert.equal(sequencer.isCurrent("codex", 1), false);
});
