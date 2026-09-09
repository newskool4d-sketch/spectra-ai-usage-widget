import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { parseBootState } from "../src/integrations/boot-state.ts";

const snapshot = {
  providerId: "codex", runtimeAvailable: true, authState: "signed-in", connectionState: "connected",
  authMethod: null, planType: "pro", source: "codex-app-server", lastSyncedAt: 1_700_000_000, bridgeInstalled: false,
  windows: [{ id: "rolling", label: "5시간 한도", usedPercent: 40, remainingPercent: 60, resetsAt: 1_700_003_600, windowDurationMins: 300 }],
  message: ""
};

describe("parseBootState", () => {
  it("accepts a well-formed boot payload", () => {
    const boot = parseBootState({ theme: "light", solid: true, standby: true, mode: "dashboard", snapshots: [snapshot] });
    assert.ok(boot);
    assert.equal(boot.theme, "light");
    assert.equal(boot.mode, "dashboard");
    assert.equal(boot.snapshots.length, 1);
    assert.equal(boot.snapshots[0].providerId, "codex");
  });

  it("rejects unknown theme or mode values", () => {
    assert.equal(parseBootState({ theme: "neon", solid: false, standby: false, mode: "mini", snapshots: [] }), null);
    assert.equal(parseBootState({ theme: "dark", solid: false, standby: false, mode: "phone", snapshots: [] }), null);
  });

  it("drops snapshots for unknown providers but keeps the rest", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: false, mode: "mini", snapshots: [snapshot, { ...snapshot, providerId: "gemini" }] });
    assert.ok(boot);
    assert.deepEqual(boot.snapshots.map(s => s.providerId), ["codex"]);
  });

  it("drops snapshots whose windows are malformed but keeps the rest", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: false, mode: "mini", snapshots: [{ ...snapshot, providerId: "claude", windows: [null] }, snapshot] });
    assert.ok(boot);
    assert.deepEqual(boot.snapshots.map(s => s.providerId), ["codex"]);
  });

  it("returns null for non-objects and missing fields", () => {
    assert.equal(parseBootState(undefined), null);
    assert.equal(parseBootState("dark"), null);
    assert.equal(parseBootState({ theme: "dark" }), null);
  });
});
