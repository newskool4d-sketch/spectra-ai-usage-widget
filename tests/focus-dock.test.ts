import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ALL_MODELS_LABEL, historyForWindow, historySegments, modelGroups, rangeStart, selectedModelGroup, summaryWindow, windowAvailable } from "../src/data/focus-dock.ts";
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
    assert.equal(modelGroups(planQuotas.codex).length, 1);
  });
  it("lists a per-model Claude limit as its own group after the account-wide windows", () => {
    const groups = modelGroups(planQuotas.claude);
    assert.deepEqual(groups.map(group => [group.id, group.label, group.windows.map(window => window.id)]),
      [["default", ALL_MODELS_LABEL, ["rolling", "weekly"]], ["fable", "Fable", ["weekly"]]]);
    // 수집 순서가 바뀌어도 계정 전체 한도가 먼저 온다.
    const reversed = modelGroups({ ...planQuotas.claude, windows: [...planQuotas.claude.windows].reverse() });
    assert.deepEqual(reversed.map(group => group.id), ["default", "fable"]);
    // 모델군이 하나뿐이면 요금제 이름이 그대로 모델군 이름이다.
    const accountOnly = { ...planQuotas.claude, windows: planQuotas.claude.windows.filter(window => !window.modelGroupId) };
    assert.deepEqual(modelGroups(accountOnly).map(group => group.label), [planQuotas.claude.planName]);
  });
  it("puts only the windows the server sends into a Claude model group (Fable has a weekly limit only)", () => {
    const fable = modelGroups(planQuotas.claude)[1];
    assert.deepEqual(fable.windows.map(window => window.id), ["weekly"]);
    // 서버가 모델별 5시간 한도를 보내면 그때만 함께 나온다.
    const own = { ...planQuotas.claude.windows[0], modelGroupId: "fable", modelGroupLabel: "Fable", usedPercent: 60, remainingPercent: 40 };
    const withOwn = modelGroups({ ...planQuotas.claude, windows: [...planQuotas.claude.windows, own] })[1];
    assert.deepEqual(withOwn.windows.map(window => [window.id, window.remainingPercent]), [["weekly", 77], ["rolling", 40]]);
  });
  it("keeps the account-wide Claude group selected by default even when the per-model limit is lower", () => {
    const fableLow = { ...planQuotas.claude, windows: planQuotas.claude.windows.map(window => window.modelGroupId ? { ...window, usedPercent: 99, remainingPercent: 1 } : window) };
    assert.equal(summaryWindow(fableLow, now)?.modelGroupId, "fable");
    assert.equal(selectedModelGroup(fableLow, undefined)?.id, "default");
    assert.equal(selectedModelGroup(fableLow, "fable")?.id, "fable");
    // 선택해 둔 모델별 한도가 응답에서 빠지면 계정 전체 한도로 돌아간다.
    const withoutFable = { ...fableLow, windows: fableLow.windows.filter(window => !window.modelGroupId) };
    assert.equal(selectedModelGroup(withoutFable, "fable")?.id, "default");
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
