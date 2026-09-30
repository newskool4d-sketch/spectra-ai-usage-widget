import type { PlanQuota, ProviderId, QuotaWindow } from "./providers";

export const providerNames: Readonly<Record<ProviderId, string>> = Object.freeze({ codex: "Codex", claude: "Claude" });
const providerIds: readonly ProviderId[] = ["codex", "claude"];

export type NextAction = Readonly<{
  recommendedProvider: ProviderId | null;
  headline: string;
  chips: readonly string[];
}>;

export function hasVerifiedUsage(quota: PlanQuota): boolean {
  return quota.confidence === "verified" && quota.windows.length > 0;
}

export function primaryWindow(quota: PlanQuota): QuotaWindow | undefined {
  return quota.windows.find(window => window.id === "rolling") ?? quota.windows[0];
}

function invalidRemaining(window: QuotaWindow): boolean {
  return !Number.isFinite(window.remainingPercent) || window.remainingPercent < 0 || window.remainingPercent > 100;
}

function needsUsageRefresh(quota: PlanQuota, now: number): boolean {
  // A passed reset is not evidence that the provider has replenished its quota.
  return quota.windows.some(window => invalidRemaining(window)
    || (window.resetsAt != null && (!Number.isFinite(window.resetsAt) || window.resetsAt <= now)));
}

function chipText(id: ProviderId, quota: PlanQuota, now: number): string {
  if (!hasVerifiedUsage(quota)) return quota.connectionState === "waiting" ? `${providerNames[id]} 첫 사용량 대기` : `${providerNames[id]} 연결 필요`;
  if (quota.windows.some(invalidRemaining)) return `${providerNames[id]} 사용량 갱신 필요`;
  // Show the binding limit, so a full rolling window cannot hide an exhausted week.
  const window = quota.windows.find(window => window.remainingPercent === 0 && (window.resetsAt == null || window.resetsAt > now))
    ?? primaryWindow(quota)!;
  const reset = window.resetsAt != null && window.resetsAt > now ? `${window.resetLabel} 초기화` : window.resetLabel;
  return `${providerNames[id]} ${window.label} ${Math.round(window.remainingPercent)}% 남음 · ${reset}`;
}

function beats(candidate: QuotaWindow, current: QuotaWindow): boolean {
  if (candidate.remainingPercent !== current.remainingPercent) return candidate.remainingPercent > current.remainingPercent;
  if (candidate.resetsAt == null || current.resetsAt == null) return false;
  return candidate.resetsAt > current.resetsAt;
}

export function computeNextAction(quotas: Readonly<Record<ProviderId, PlanQuota>>, now: number): NextAction {
  const chips = providerIds.map(id => chipText(id, quotas[id], now));
  const connected = providerIds.filter(id => hasVerifiedUsage(quotas[id]));
  if (connected.length === 0) {
    return { recommendedProvider: null, headline: `${providerIds.map(id => providerNames[id]).join("·")} 연결 후 권장 서비스를 알려 드립니다`, chips };
  }
  const available = connected.filter(id => !needsUsageRefresh(quotas[id], now)
    && quotas[id].windows.every(window => window.remainingPercent > 0));
  if (available.length === 0) {
    const headline = connected.some(id => needsUsageRefresh(quotas[id], now))
      ? "사용량을 새로고침한 뒤 권장 서비스를 확인해 주세요"
      : "사용 가능한 한도가 없습니다 · 초기화를 기다려 주세요";
    return { recommendedProvider: null, headline, chips };
  }
  let best = available[0];
  for (const id of available.slice(1)) {
    const candidate = primaryWindow(quotas[id]);
    const current = primaryWindow(quotas[best]);
    if (candidate && current && beats(candidate, current)) best = id;
  }
  return { recommendedProvider: best, headline: `지금은 ${providerNames[best]}에서 작업`, chips };
}

export type Pace = "fast" | "steady" | "even";
export type TimeProgress = Readonly<{ timePercent: number; pace: Pace }>;

export function computeTimeProgress(
  window: Pick<QuotaWindow, "usedPercent" | "resetsAt" | "windowDurationMins">,
  now: number
): TimeProgress | null {
  if (window.resetsAt == null || window.windowDurationMins == null || window.windowDurationMins <= 0) return null;
  if (window.resetsAt <= now) return null;
  const durationMs = window.windowDurationMins * 60 * 1000;
  const elapsed = now - (window.resetsAt - durationMs);
  const timePercent = Math.max(0, Math.min(100, (elapsed / durationMs) * 100));
  const gap = window.usedPercent - timePercent;
  return { timePercent, pace: gap > 10 ? "fast" : gap < -10 ? "steady" : "even" };
}

export function paceLabel(pace: Pace): string {
  return pace === "fast" ? "빠름" : pace === "steady" ? "여유" : "보통";
}
