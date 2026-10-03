import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { CLAUDE_TOKEN_EXPIRED, LOOKUP_DELAYED, autoRefreshLabel, claudeFreshness } from "../src/data/usage-freshness.ts";
import { planQuotas, type PlanQuota } from "../src/data/providers.ts";

const captured = 1_800_000_000_000;
const quota: PlanQuota = {
  ...planQuotas.claude,
  source: "claude-usage-api",
  confidence: "verified",
  connectionState: "connected",
  lastSyncedAt: "2027. 01. 15. 17:00",
  lastSyncedAtMs: captured,
  statusMessage: "",
  windows: [{ ...planQuotas.claude.windows[0], resetsAt: captured + 60_000 }]
};

describe("Claude freshness", () => {
  it("shows the capture time, not the current refresh attempt time", () => {
    const fresh = claudeFreshness(quota, captured + 30_000);
    assert.equal(fresh.label, "동기화됨");
    assert.ok(fresh.tooltip.includes(`마지막 동기화: ${quota.lastSyncedAt} (데이터 수집 기준)`));
    const stale = claudeFreshness({ ...quota, connectionState: "stale", statusMessage: "서버 연결 실패" }, captured + 40_000);
    assert.equal(stale.label, "갱신 대기 (캐시)");
    assert.ok(stale.tooltip.includes(quota.lastSyncedAt!));
    assert.ok(stale.tooltip.includes("서버 연결 실패"));
  });

  it("marks reset expiry and fallback caches as pending before any new response", () => {
    assert.equal(claudeFreshness(quota, captured + 59_999).label, "동기화됨");
    assert.equal(claudeFreshness(quota, captured + 60_000).label, "갱신 대기 (캐시)");
    assert.equal(claudeFreshness({ ...quota, source: "claude-statusline" }, captured).label, "갱신 대기 (캐시)");
  });

  it("does not claim freshness for missing, future or older-than-24h capture times", () => {
    const noReset = { ...quota, windows: [{ ...quota.windows[0], resetsAt: null }] };
    assert.equal(claudeFreshness(noReset, captured + 86_400_000).label, "동기화됨");
    assert.equal(claudeFreshness(noReset, captured + 86_400_001).label, "갱신 대기 (캐시)");
    for (const value of [null, undefined, NaN, Infinity, captured + 1]) {
      assert.equal(claudeFreshness({ ...noReset, lastSyncedAtMs: value }, captured).label, "갱신 대기 (캐시)");
    }
  });

  it("distinguishes login, first-use waiting, errors and installation without fake sync times", () => {
    for (const [state, label] of [
      ["signed_out", "로그인 필요"], ["waiting", "사용량 갱신 대기"],
      ["error", "사용량 확인 지연"], ["not_installed", "Claude Code 설치 필요"]
    ] as const) {
      const result = claudeFreshness({ ...quota, connectionState: state, confidence: "unavailable", source: "unavailable", windows: [], lastSyncedAt: null, lastSyncedAtMs: null }, captured);
      assert.equal(result.label, label);
      assert.ok(result.tooltip.includes("마지막 동기화: 기록 없음"));
    }
  });

  it("reads a failed lookup as delayed, with the same text as the taskbar strip", () => {
    // taskbar_strip::LOOKUP_DELAYED pins the same string on the Rust side.
    assert.equal(LOOKUP_DELAYED, "사용량 확인 지연");
    const failed = claudeFreshness({ ...quota, connectionState: "error", confidence: "unavailable", source: "unavailable", windows: [], liveFailure: "claude-auth-status-failed", statusMessage: "Claude Code 응답이 없어 이번 확인을 마치지 못했습니다." }, captured);
    assert.equal(failed.label, LOOKUP_DELAYED);
    assert.ok(!failed.tooltip.includes("계정 상태"));
    const carried = claudeFreshness({ ...quota, connectionState: "stale", liveFailure: "claude-auth-status-failed", statusMessage: "Claude Code 응답이 없어 이번 확인을 마치지 못했습니다. 마지막 동기화 값을 표시합니다." }, captured + 30_000);
    assert.equal(carried.label, "갱신 대기 (캐시)", "carried values keep the cache label");
  });

  it("keeps browser demo values explicitly labelled as demo", () => {
    assert.equal(claudeFreshness(planQuotas.claude, captured).label, "브라우저 데모");
  });

  it("shows login recovery instructions on a cache even when the CLI account is connected", () => {
    for (const statusMessage of ["Claude Code 로그인 갱신이 필요합니다.", "Claude 사용량 조회를 위한 로그인이 필요합니다."]) {
      const result = claudeFreshness({ ...quota, accountLabel: "공식 계정 연결 확인", connectionState: "stale", source: "claude-statusline", statusMessage }, captured);
      assert.equal(result.label, "갱신 대기 (캐시)");
      assert.ok(result.tooltip.includes(statusMessage));
      assert.ok(result.tooltip.includes(quota.lastSyncedAt!));
    }
  });

  it("asks for a login refresh when the Claude Code token expired, keeping the cached usage", () => {
    const expired = claudeFreshness({ ...quota, connectionState: "stale", source: "claude-statusline", liveFailure: CLAUDE_TOKEN_EXPIRED, statusMessage: "Claude Code 로그인 갱신이 필요합니다. 토큰이 만료되어 Claude Code를 한 번 실행해 주세요." }, captured);
    assert.equal(expired.label, "로그인 갱신 필요");
    assert.ok(expired.tooltip.includes("토큰이 만료되어"));
    assert.equal(claudeFreshness({ ...quota, liveFailure: null }, captured + 30_000).label, "동기화됨");
  });

  it("labels a native auto refresh with the wall-clock time", () => {
    assert.equal(autoRefreshLabel(new Date(2027, 0, 15, 9, 5)), "자동 확인 09:05");
    assert.equal(autoRefreshLabel(new Date(2027, 0, 15, 21, 40)), "자동 확인 21:40");
  });
});
