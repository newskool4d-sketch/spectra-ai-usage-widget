import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { nextConnectionAction } from "../src/integrations/provider-connect.ts";

describe("provider connection routing", () => {
  it("installs the Antigravity bridge before requiring auth that only its first payload can establish", () => {
    for (const authState of ["unknown", "signed-out", "signed-in"] as const) {
      const snapshot = { runtimeAvailable: true, authState, bridgeInstalled: false };
      assert.equal(nextConnectionAction("antigravity", snapshot, "install-bridge"), "install-bridge");
      assert.equal(nextConnectionAction("antigravity", snapshot, "refresh"), "refresh");
    }
  });
  it("waits for a first CLI payload after bridge installation without repeatedly launching login", () => {
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: true, authState: "unknown", bridgeInstalled: true }, "refresh"), "refresh");
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: true, authState: "signed-out", bridgeInstalled: true }, "login"), "login");
  });
  it("does not install a bridge without its runtime and preserves Claude login before bridge setup", () => {
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: false, authState: "unknown", bridgeInstalled: false }, "install-bridge"), "install-cli");
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-out", bridgeInstalled: false }, "login"), "login");
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-in", bridgeInstalled: false }, "install-bridge"), "install-bridge");
    assert.equal(nextConnectionAction("codex", { runtimeAvailable: true, authState: "signed-in", bridgeInstalled: false }, "refresh"), "refresh");
  });

  it("keeps a Claude refresh read-only when the bridge is absent or the session signed out", () => {
    for (const authState of ["signed-in", "signed-out"] as const) {
      assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState, bridgeInstalled: false }, "refresh"), "refresh");
    }
  });

  it("does not replace the requested action when the Claude state changes during refresh", () => {
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-in", bridgeInstalled: false }, "login"), "refresh");
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-out", bridgeInstalled: false }, "install-bridge"), "refresh");
  });
});
