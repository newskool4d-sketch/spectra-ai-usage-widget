import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { computeTimeProgress, paceLabel } from "../src/data/next-action.ts";

const now = 1_700_000_000_000;
const minute = 60 * 1000;

describe("computeTimeProgress", () => {
  it("returns null without reset time or duration", () => {
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: null, windowDurationMins: 300 }, now), null);
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: now + 60 * minute, windowDurationMins: null }, now), null);
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: now + 60 * minute, windowDurationMins: 0 }, now), null);
  });

  it("derives elapsed share from resetsAt minus duration", () => {
    const result = computeTimeProgress({ usedPercent: 50, resetsAt: now + 150 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 50);
    assert.equal(result?.pace, "even");
  });

  it("flags fast when usage runs more than 10 points ahead of time", () => {
    const result = computeTimeProgress({ usedPercent: 50, resetsAt: now + 240 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 20);
    assert.equal(result?.pace, "fast");
  });

  it("flags steady when time runs more than 10 points ahead of usage", () => {
    const result = computeTimeProgress({ usedPercent: 20, resetsAt: now + 60 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 80);
    assert.equal(result?.pace, "steady");
  });

  it("returns null once the reset time has passed and clamps future windows to 0", () => {
    assert.equal(computeTimeProgress({ usedPercent: 0, resetsAt: now - minute, windowDurationMins: 300 }, now), null);
    assert.equal(computeTimeProgress({ usedPercent: 0, resetsAt: now + 400 * minute, windowDurationMins: 300 }, now)?.timePercent, 0);
  });

  it("returns null for a window whose reset already passed", () => {
    assert.equal(computeTimeProgress({ usedPercent: 30, resetsAt: now - minute, windowDurationMins: 300 }, now), null);
  });

  it("labels each pace in Korean", () => {
    assert.deepEqual([paceLabel("fast"), paceLabel("steady"), paceLabel("even")], ["빠름", "여유", "보통"]);
  });
});
