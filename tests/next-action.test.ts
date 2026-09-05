import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { computeNextAction } from "../src/data/next-action.ts";
import type { PlanQuota, ProviderId, QuotaWindow } from "../src/data/providers.ts";

const base = 1_700_000_000_000;
const hour = 60 * 60 * 1000;

function window(overrides: Partial<QuotaWindow> = {}): QuotaWindow {
  return { id: "rolling", label: "5시간 한도", usedPercent: 60, remainingPercent: 40, resetLabel: "2시간 후", kindLabel: "", resetsAt: base + 2 * hour, windowDurationMins: 300, ...overrides };
}

function connected(providerId: ProviderId, windows: readonly QuotaWindow[] = [window()]): PlanQuota {
  return { providerId, planName: "Test", accountLabel: "", authMethod: "provider-delegated", connectionState: "connected", source: "claude-statusline", confidence: "verified", lastSyncedAt: null, bridgeInstalled: true, statusMessage: "", windows };
}

function pending(providerId: ProviderId): PlanQuota {
  return { ...connected(providerId), connectionState: "not_connected", source: "unavailable", confidence: "unavailable", windows: [window({ usedPercent: 0, remainingPercent: 0, resetLabel: "연결 후 표시", resetsAt: null, windowDurationMins: null })] };
}

describe("computeNextAction", () => {
  it("recommends the provider whose primary window keeps more remaining percent", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ usedPercent: 76, remainingPercent: 24, resetLabel: "1시간 18분 후" })]),
      claude: connected("claude", [window({ usedPercent: 68, remainingPercent: 32, resetLabel: "3시간 후" })])
    }, base);
    assert.equal(result.recommendedProvider, "claude");
    assert.equal(result.headline, "지금은 Claude에서 작업");
    assert.deepEqual(result.chips, ["Codex 5시간 한도 24% 남음 · 1시간 18분 후 초기화", "Claude 5시간 한도 32% 남음 · 3시간 후 초기화"]);
  });

  it("breaks a tie by the later reset time", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 50, resetsAt: base + 1 * hour })]),
      claude: connected("claude", [window({ remainingPercent: 50, resetsAt: base + 3 * hour })])
    }, base);
    assert.equal(result.recommendedProvider, "claude");
  });

  it("keeps provider order when tied and reset time is unknown", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 50, resetsAt: null })]),
      claude: connected("claude", [window({ remainingPercent: 50, resetsAt: null })])
    }, base);
    assert.equal(result.recommendedProvider, "codex");
  });

  it("keeps provider order when only one tied window has a known reset", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 50, resetsAt: null })]),
      claude: connected("claude", [window({ remainingPercent: 50, resetsAt: base + 3 * hour })])
    }, base);
    assert.equal(result.recommendedProvider, "codex");
  });

  it("uses the rolling window even when it is not first", () => {
    const weekly = window({ id: "weekly", label: "주간 한도", remainingPercent: 90, windowDurationMins: 10080 });
    const result = computeNextAction({
      codex: connected("codex", [weekly, window({ remainingPercent: 10 })]),
      claude: connected("claude", [window({ remainingPercent: 30 })])
    }, base);
    assert.equal(result.recommendedProvider, "claude");
  });

  it("recommends codex when it keeps more remaining", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 60 })]),
      claude: connected("claude", [window({ remainingPercent: 40 })])
    }, base);
    assert.equal(result.recommendedProvider, "codex");
    assert.equal(result.headline, "지금은 Codex에서 작업");
  });

  it("asks to connect when no provider has verified usage", () => {
    const result = computeNextAction({ codex: pending("codex"), claude: pending("claude") }, base);
    assert.equal(result.recommendedProvider, null);
    assert.equal(result.headline, "Codex·Claude 연결 후 권장 서비스를 알려 드립니다");
    assert.deepEqual(result.chips, ["Codex 연결 필요", "Claude 연결 필요"]);
  });

  it("recommends the only connected provider and marks the other as needing connection", () => {
    const result = computeNextAction({ codex: pending("codex"), claude: connected("claude") }, base);
    assert.equal(result.recommendedProvider, "claude");
    assert.equal(result.chips[0], "Codex 연결 필요");
  });

  it("treats example-confidence demo data as unconnected", () => {
    const result = computeNextAction({
      codex: { ...connected("codex"), source: "example", confidence: "example" },
      claude: { ...connected("claude"), source: "example", confidence: "example" }
    }, base);
    assert.equal(result.recommendedProvider, null);
    assert.deepEqual(result.chips, ["Codex 연결 필요", "Claude 연결 필요"]);
  });

  it("labels a signed-in provider waiting for first usage", () => {
    const result = computeNextAction({
      codex: { ...pending("codex"), connectionState: "waiting" },
      claude: connected("claude")
    }, base);
    assert.equal(result.chips[0], "Codex 첫 사용량 대기");
  });

  it("does not append 초기화 to sentinel reset labels", () => {
    const claudeResult = computeNextAction({
      codex: pending("codex"),
      claude: connected("claude", [window({ resetLabel: "초기화 시각 확인 중", resetsAt: null })])
    }, base);
    assert.equal(claudeResult.chips[1], "Claude 5시간 한도 40% 남음 · 초기화 시각 확인 중");

    const codexResult = computeNextAction({
      codex: connected("codex", [window({ resetLabel: "초기화 정보 갱신 필요", resetsAt: base - hour })]),
      claude: pending("claude")
    }, base);
    assert.equal(codexResult.chips[0], "Codex 5시간 한도 40% 남음 · 초기화 정보 갱신 필요");
  });
});
