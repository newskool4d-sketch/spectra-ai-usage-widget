import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { parseBootState, providersMissingFromBoot, providersToRefreshAfterBoot } from "../src/integrations/boot-state.ts";

const snapshot = {
  providerId: "codex", runtimeAvailable: true, authState: "signed-in", connectionState: "connected",
  authMethod: null, planType: "pro", source: "codex-app-server", lastSyncedAt: 1_700_000_000, bridgeInstalled: false,
  windows: [{ id: "rolling", label: "5시간 한도", usedPercent: 40, remainingPercent: 60, resetsAt: 1_700_003_600, windowDurationMins: 300 }],
  message: ""
};

describe("parseBootState", () => {
  it("defaults strip to false for absent or non-boolean values", () => {
    for (const strip of [undefined, null, "true", 1]) {
      const boot = parseBootState({ theme: "dark", solid: false, standby: true, strip, mode: "mini", snapshots: [snapshot] });
      assert.equal(boot?.strip, false);
      assert.equal(boot?.standby, true);
      assert.equal(boot?.snapshots.length, 1);
    }
  });

  it("restores an explicit strip preference in either window mode", () => {
    for (const mode of ["mini", "dashboard"]) {
      for (const strip of [false, true]) {
        const boot = parseBootState({ theme: "light", solid: true, standby: true, strip, mode, snapshots: [snapshot] });
        assert.equal(boot?.strip, strip);
        assert.equal(boot?.mode, mode);
        assert.deepEqual(boot?.snapshots, [snapshot]);
      }
    }
  });

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

describe("providersMissingFromBoot", () => {
  const all = ["codex", "claude"] as const;

  it("lists every provider when there is no boot state", () => {
    assert.deepEqual(providersMissingFromBoot(null, all), ["codex", "claude"]);
  });

  it("lists only the providers absent from the boot snapshots", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [snapshot] });
    assert.deepEqual(providersMissingFromBoot(boot, all), ["claude"]);
  });

  it("lists nothing when every provider was restored", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [snapshot, { ...snapshot, providerId: "claude" }] });
    assert.deepEqual(providersMissingFromBoot(boot, all), []);
  });
});

describe("providersToRefreshAfterBoot", () => {
  const all = ["codex", "claude"] as const;
  const now = snapshot.lastSyncedAt * 1000;

  it("refreshes a restored Claude snapshot in addition to missing providers", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [snapshot, { ...snapshot, providerId: "claude" }] });
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["claude"]);
  });

  it("does not add Claude when it was not restored", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [snapshot] });
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["claude"]);
  });

  it("refreshes every provider on a cold boot", () => {
    assert.deepEqual(providersToRefreshAfterBoot(null, all, now), ["codex", "claude"]);
  });

  it("reuses fresh Codex usage but refreshes at the native two-minute interval", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, strip: false, mode: "mini", snapshots: [snapshot, { ...snapshot, providerId: "claude" }] });
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now + 119_999), ["claude"]);
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now + 120_000), ["codex", "claude"]);
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now + 86_400_000), ["codex", "claude"]);
  });

  it("refreshes Codex when a rolling or weekly reset has elapsed despite a recent capture", () => {
    for (const id of ["rolling", "weekly"]) {
      const expired = { ...snapshot, lastSyncedAt: now / 1000 - 1, windows: [snapshot.windows[0], { ...snapshot.windows[0], id, resetsAt: now / 1000 }] };
      const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [expired] });
      assert.deepEqual(providersToRefreshAfterBoot(boot, all, now - 1), ["claude"]);
      assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["codex", "claude"]);
    }
  });

  it("refreshes Codex for missing, non-finite or future capture times", () => {
    for (const lastSyncedAt of [null, undefined, NaN, Infinity, now / 1000 + 1]) {
      const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [{ ...snapshot, lastSyncedAt }] });
      assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["codex", "claude"]);
    }
  });

  it("refreshes Codex when the restored lookup failed or has no quota windows", () => {
    for (const restored of [{ ...snapshot, connectionState: "error" }, { ...snapshot, connectionState: "stale" }, { ...snapshot, windows: [] }]) {
      const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [restored] });
      assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["codex", "claude"]);
    }
  });

  it("does not require a known reset time to reuse fresh Codex usage", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: true, mode: "mini", snapshots: [{ ...snapshot, windows: [{ ...snapshot.windows[0], resetsAt: null }] }] });
    assert.deepEqual(providersToRefreshAfterBoot(boot, all, now), ["claude"]);
  });
});
