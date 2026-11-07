import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { historyForWindow, historySegments, modelGroups, rangeStart, selectedModelGroup, summaryWindow, windowAvailable } from "../src/data/focus-dock.ts";
import { planQuotas, metricLabels } from "../src/data/providers.ts";
import { parseBootState, providersToRefreshAfterBoot } from "../src/integrations/boot-state.ts";
import { antigravityFreshness } from "../src/data/usage-freshness.ts";
import type { UsageHistoryPoint } from "../src/integrations/tauri-native-bridge.ts";

const now = new Date(2026, 10, 2, 12).getTime();
const point = (overrides: Partial<UsageHistoryPoint> = {}): UsageHistoryPoint => ({ providerId: "antigravity", modelGroupId: "gemini", modelGroupLabel: "Gemini", windowId: "weekly", label: "주간 한도", capturedAt: now / 1000 - 60, remainingPercent: 83, usedPercent: 17, resetsAt: null, windowDurationMins: 10080, ...overrides });

describe("focus dock models", () => {
  it("keeps model-group quota windows distinct and never fabricates an Argon rolling window", () => {
    const groups = modelGroups(planQuotas.antigravity);
    assert.deepEqual(groups.map(group => group.id), ["gemini", "gemini-4-argon", "claude-gpt"]);
    const reversed = modelGroups({ ...planQuotas.antigravity, windows: [...planQuotas.antigravity.windows].reverse() });
    assert.deepEqual(reversed.map(group => group.id), ["gemini", "gemini-4-argon", "claude-gpt"]);
    assert.deepEqual(groups.find(group => group.id === "gemini-4-argon")?.windows.map(window => window.id), ["weekly"]);
    assert.deepEqual(groups.find(group => group.id === "claude-gpt")?.windows.map(window => window.id), ["rolling", "weekly"]);
    assert.equal(modelGroups(planQuotas.claude).length, 1);
  });
  it("defaults to the constrained model group and follows an explicit selection", () => {
    assert.equal(selectedModelGroup(planQuotas.antigravity, undefined)?.id, "gemini-4-argon");
    assert.equal(selectedModelGroup(planQuotas.antigravity, "gemini")?.id, "gemini");
    assert.equal(selectedModelGroup(planQuotas.antigravity, "removed-group")?.id, "gemini-4-argon");
  });
  it("does not show expired, unverified, invalid or missing windows as usable quota", () => {
    const original = planQuotas.codex.windows[0];
    for (const remainingPercent of [-1, 101, NaN, Infinity]) assert.equal(windowAvailable(planQuotas.codex, { ...original, remainingPercent }, now), false);
    assert.equal(windowAvailable(planQuotas.codex, { ...original, resetsAt: now }, now), false);
    assert.equal(windowAvailable({ ...planQuotas.codex, confidence: "unavailable" }, original, now), false);
    assert.equal(windowAvailable(planQuotas.codex, undefined, now), false);
    assert.equal(summaryWindow({ ...planQuotas.codex, windows: [] }, now), undefined);
  });
  it("offers subscription model details rather than monetary estimates", () => {
    assert.deepEqual(Object.keys(metricLabels), ["remaining", "subscription", "requests"]);
    assert.equal(metricLabels.subscription, "구독 모델");
  });
  it("ages Antigravity CLI data after a day or a reset even without a new native snapshot", () => {
    const quota = { ...planQuotas.antigravity, source: "antigravity-statusline" as const, confidence: "verified" as const, connectionState: "connected" as const, bridgeInstalled: true, lastSyncedAtMs: now };
    assert.equal(antigravityFreshness(quota, now + 300_000), "CLI 수집값");
    assert.equal(antigravityFreshness(quota, now + 86_399_999), "CLI 수집값");
    assert.equal(antigravityFreshness(quota, now + 86_400_000), "CLI 실행 후 갱신 필요");
    assert.equal(antigravityFreshness({ ...quota, windows: [{ ...quota.windows[0], resetsAt: now }] }, now), "CLI 실행 후 갱신 필요");
    assert.equal(antigravityFreshness({ ...quota, lastSyncedAtMs: now + 1 }, now), "CLI 실행 후 갱신 필요");
    assert.equal(antigravityFreshness({ ...quota, windows: [], confidence: "unavailable" }, now), "CLI 첫 실행 대기");
    assert.equal(antigravityFreshness({ ...quota, bridgeInstalled: false }, now), "CLI 연결 설정 필요");
  });
});

describe("history range and data fidelity", () => {
  it("uses local calendar midnight and includes today in seven and thirty day ranges", () => {
    assert.equal(rangeStart("24H", now), new Date(2026, 10, 2).getTime());
    assert.equal(rangeStart("7D", now), new Date(2026, 9, 27).getTime());
    assert.equal(rangeStart("30D", now), new Date(2026, 9, 4).getTime());
  });
  it("filters by provider, model group, quota window, time and valid percentage", () => {
    const correct = point();
    const sevenDaysOnly = point({ capturedAt: new Date(2026, 9, 28, 12).getTime() / 1000 });
    const thirtyDaysOnly = point({ capturedAt: new Date(2026, 9, 10, 12).getTime() / 1000 });
    const samples = [point({ providerId: "claude" }), point({ modelGroupId: "claude-gpt" }), point({ windowId: "rolling" }), point({ remainingPercent: NaN }), point({ capturedAt: now / 1000 + 1 }), thirtyDaysOnly, sevenDaysOnly, correct];
    assert.deepEqual(historyForWindow(samples, "antigravity", "gemini", "weekly", "24H", now), [correct]);
    assert.deepEqual(historyForWindow(samples, "antigravity", "gemini", "weekly", "7D", now), [sevenDaysOnly, correct]);
    assert.deepEqual(historyForWindow(samples, "antigravity", "gemini", "weekly", "30D", now), [thirtyDaysOnly, sevenDaysOnly, correct]);
  });
  it("shows a single sample and separates unsampled gaps and reset boundaries", () => {
    assert.equal(historySegments([point()])[0].length, 1);
    const samples = [point({ capturedAt: 1 }), point({ capturedAt: 60 }), point({ capturedAt: 20000 }), point({ capturedAt: 20060, resetsAt: 90000 })];
    assert.deepEqual(historySegments(samples).map(segment => segment.length), [2, 1, 1]);
    assert.deepEqual(historyForWindow([], "codex", "default", "weekly", "30D", now), []);
  });
  it("restores Antigravity boot snapshots and refreshes the official statusline cache on reopening", () => {
    const snapshot = { providerId: "antigravity", connectionState: "connected", source: "antigravity-statusline", windows: [{ id: "weekly", label: "주간 한도", modelGroupId: "gemini", usedPercent: 17, remainingPercent: 83 }] };
    const boot = parseBootState({ theme: "light", solid: false, standby: false, mode: "mini", snapshots: [snapshot] });
    assert.equal(boot?.snapshots.length, 1);
    assert.deepEqual(providersToRefreshAfterBoot(boot, ["antigravity"], now), ["antigravity"]);
  });
});
