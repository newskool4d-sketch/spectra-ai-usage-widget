import { afterEach, beforeEach, describe, it } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { registerHooks } from "node:module";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";

// Resolve the app's bundler-style local imports while running its actual adapter
// and the installed Tauri JS plugins in Node. Only the native IPC boundary is fake.
const sourceResolution = registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier.startsWith(".") && context.parentURL) {
      const candidate = new URL(`${specifier}.ts`, context.parentURL);
      if (existsSync(candidate)) return nextResolve(candidate.href, context);
    }
    return nextResolve(specifier, context);
  },
});
const { checkForAppUpdate } = await import("../src/integrations/app-updater.ts");
sourceResolution.deregister();

type NativeCall = { command: string; payload: Record<string, unknown> | undefined };
const metadata = { rid: 7, currentVersion: "0.2.6", version: "0.2.7", body: "Release notes", rawJson: {} };
let calls: NativeCall[];

function bridge(handler: (command: string, payload: Record<string, unknown> | undefined) => unknown) {
  mockIPC((command, payload) => {
    calls.push({ command, payload });
    return handler(command, payload as Record<string, unknown> | undefined);
  });
}

beforeEach(() => {
  calls = [];
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { crypto: globalThis.crypto, __TAURI__: { core: { invoke: () => Promise.resolve() } } },
  });
});

afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(globalThis, "window");
});

describe("app updater through the installed Tauri JS plugins", () => {
  it("does not make an update request outside the native runtime", async () => {
    Reflect.deleteProperty(window, "__TAURI__");
    bridge(() => { throw new Error("must not invoke IPC"); });
    assert.equal(await checkForAppUpdate(), null);
    assert.deepEqual(calls, []);
  });

  it("sends a finite manifest timeout and preserves the no-update result", async () => {
    bridge(() => null);
    assert.equal(await checkForAppUpdate(), null);
    assert.equal(calls[0].command, "plugin:updater|check");
    assert.equal(calls[0].payload?.timeout, 15_000);
  });

  it("propagates a failed manifest request and permits the next check", async () => {
    let attempts = 0;
    bridge(() => {
      if (++attempts === 1) throw new Error("manifest request timed out");
      return metadata;
    });
    await assert.rejects(checkForAppUpdate(), /manifest request timed out/);
    const update = await checkForAppUpdate();
    assert.equal(update?.version, "0.2.7");
    assert.equal(update?.notes, "Release notes");
    assert.equal(attempts, 2);
  });

  it("waits for installation success before restarting and gives download its own timeout", async () => {
    const install = Promise.withResolvers<void>();
    bridge(command => command === "plugin:updater|check" ? metadata
      : command === "plugin:updater|download_and_install" ? install.promise : undefined);
    const update = await checkForAppUpdate();
    assert.ok(update);
    const completion = update.install();
    await Promise.resolve();
    const download = calls.find(call => call.command === "plugin:updater|download_and_install");
    assert.equal(download?.payload?.rid, 7);
    assert.equal(download?.payload?.timeout, 120_000);
    assert.equal(calls.some(call => call.command === "plugin:process|restart"), false);
    install.resolve();
    await completion;
    assert.equal(calls.at(-1)?.command, "plugin:process|restart");
  });

  it("propagates a failed download without restarting and retries the same update", async () => {
    let attempts = 0;
    bridge(command => {
      if (command === "plugin:updater|check") return metadata;
      if (command === "plugin:updater|download_and_install" && ++attempts === 1) {
        throw new Error("download request timed out");
      }
    });
    const update = await checkForAppUpdate();
    assert.ok(update);
    await assert.rejects(update.install(), /download request timed out/);
    assert.equal(calls.some(call => call.command === "plugin:process|restart"), false);
    await update.install();
    assert.equal(attempts, 2);
    assert.equal(calls.filter(call => call.command === "plugin:process|restart").length, 1);
  });

  it("coalesces concurrent and repeated successful installs of one update", async () => {
    const install = Promise.withResolvers<void>();
    bridge(command => command === "plugin:updater|check" ? metadata
      : command === "plugin:updater|download_and_install" ? install.promise : undefined);
    const update = await checkForAppUpdate();
    assert.ok(update);
    const first = update.install();
    const second = update.install();
    install.resolve();
    await Promise.all([first, second]);
    await update.install();
    assert.equal(calls.filter(call => call.command === "plugin:updater|download_and_install").length, 1);
    assert.equal(calls.filter(call => call.command === "plugin:process|restart").length, 1);
  });

  it("retries only restart if installation succeeded but restart failed", async () => {
    let restartAttempts = 0;
    bridge(command => {
      if (command === "plugin:updater|check") return metadata;
      if (command === "plugin:process|restart" && ++restartAttempts === 1) throw new Error("restart failed");
    });
    const update = await checkForAppUpdate();
    assert.ok(update);
    await assert.rejects(update.install(), /restart failed/);
    await update.install();
    assert.equal(restartAttempts, 2);
    assert.equal(calls.filter(call => call.command === "plugin:updater|download_and_install").length, 1);
  });
});
