import type { PlanQuota, QuotaWindow, UsageRange } from "./providers";
import type { UsageHistoryPoint } from "../integrations/tauri-native-bridge";

export type ModelGroup = Readonly<{ id: string; label: string; windows: readonly QuotaWindow[] }>;
export const groupId = (window: Pick<QuotaWindow, "modelGroupId">) => window.modelGroupId || "default";

// Antigravity 모델군 표시 순서. 목록에 없는 모델군은 indexOf의 -1이 `>>> 0`으로 최댓값이 되어 수집 순서대로 뒤에 온다.
const groupOrder = ["gemini", "gemini-4-argon", "claude-gpt"];
const groupRank = (id: string) => groupOrder.indexOf(id) >>> 0;

export function modelGroups(quota: PlanQuota): ModelGroup[] {
  const groups = new Map<string, ModelGroup>();
  for (const window of quota.windows) {
    const id = groupId(window);
    const previous = groups.get(id);
    groups.set(id, { id, label: window.modelGroupLabel || quota.planName, windows: [...(previous?.windows ?? []), window] });
  }
  return [...groups.values()].sort((a, b) => groupRank(a.id) - groupRank(b.id));
}

export function windowAvailable(quota: PlanQuota, window: QuotaWindow | undefined, now = Date.now()): boolean {
  return quota.confidence !== "unavailable" && window !== undefined
    && Number.isFinite(window.remainingPercent) && window.remainingPercent >= 0 && window.remainingPercent <= 100
    && !(window.resetsAt != null && window.resetsAt <= now);
}

/** The most constrained current window is a useful service summary, not a cross-provider recommendation. */
export function summaryWindow(quota: PlanQuota, now = Date.now()): QuotaWindow | undefined {
  return quota.windows.filter(window => windowAvailable(quota, window, now))
    .reduce<QuotaWindow | undefined>((best, window) => !best || window.remainingPercent < best.remainingPercent ? window : best, undefined);
}

export function selectedModelGroup(quota: PlanQuota, selected: string | undefined): ModelGroup | undefined {
  const groups = modelGroups(quota);
  return groups.find(group => group.id === selected)
    ?? groups.find(group => group.id === groupId(summaryWindow(quota) ?? {}))
    ?? groups[0];
}

export function rangeStart(range: UsageRange, now = Date.now()): number {
  const date = new Date(now);
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() - (range === "30D" ? 29 : range === "7D" ? 6 : 0));
  return date.getTime();
}

export function historyForWindow(points: readonly UsageHistoryPoint[], providerId: string, modelGroupId: string, windowId: string, range: UsageRange, now = Date.now()): UsageHistoryPoint[] {
  const from = rangeStart(range, now) / 1000;
  const to = now / 1000;
  return points.filter(point => point.providerId === providerId && (point.modelGroupId || "default") === modelGroupId
    && point.windowId === windowId && Number.isFinite(point.capturedAt) && point.capturedAt >= from && point.capturedAt <= to
    && Number.isFinite(point.remainingPercent) && point.remainingPercent >= 0 && point.remainingPercent <= 100)
    .sort((a, b) => a.capturedAt - b.capturedAt);
}

/** Preserve time gaps; a missing period must not become a zero or an interpolated usage record. */
export function historySegments(points: readonly UsageHistoryPoint[]): readonly UsageHistoryPoint[][] {
  const segments: UsageHistoryPoint[][] = [];
  for (const point of points) {
    const tail = segments.at(-1);
    if (!tail || point.capturedAt - tail[tail.length - 1].capturedAt > 3 * 60 * 60
      || point.resetsAt !== tail[tail.length - 1].resetsAt) segments.push([point]);
    else tail.push(point);
  }
  return segments;
}
