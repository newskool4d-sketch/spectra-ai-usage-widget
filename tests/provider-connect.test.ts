import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { nextConnectionAction } from "../src/integrations/provider-connect.ts";

describe("provider connection routing", () => {
  it("installs the Antigravity bridge before requiring auth that only its first payload can establish", () => {
    for (const authState of ["unknown", "signed-out", "signed-in"] as const) assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: true, authState, bridgeInstalled: false }), "install-bridge");
  });
  it("waits for a first CLI payload after bridge installation without repeatedly launching login", () => {
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: true, authState: "unknown", bridgeInstalled: true }), "refresh");
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: true, authState: "signed-out", bridgeInstalled: true }), "login");
  });
  it("does not install a bridge without its runtime and preserves Claude login before bridge setup", () => {
    assert.equal(nextConnectionAction("antigravity", { runtimeAvailable: false, authState: "unknown", bridgeInstalled: false }), "install-cli");
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-out", bridgeInstalled: false }), "login");
    assert.equal(nextConnectionAction("claude", { runtimeAvailable: true, authState: "signed-in", bridgeInstalled: false }), "install-bridge");
    assert.equal(nextConnectionAction("codex", { runtimeAvailable: true, authState: "signed-in", bridgeInstalled: false }), "refresh");
  });
});
