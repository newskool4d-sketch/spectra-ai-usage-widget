import type { PlanQuota, QuotaWindow, UsageRange } from "./providers";
import type { UsageHistoryPoint } from "../integrations/tauri-native-bridge";

export type ModelGroup = Readonly<{ id: string; label: string; windows: readonly QuotaWindow[] }>;
const DEFAULT_GROUP_ID = "default";
export const groupId = (window: Pick<QuotaWindow, "modelGroupId">) => window.modelGroupId || DEFAULT_GROUP_ID;
/** 모델별 한도(Claude의 Fable 등)가 함께 있을 때 모델군 없는 계정 전체 한도에 붙이는 이름. */
export const ALL_MODELS_LABEL = "전체 모델";
/** 모델군 보기에 함께 보여 주는 계정 전체 한도에 붙이는 표기. */
export const ACCOUNT_WIDE_NOTE = "전체 모델 공용";

// 모델군 표시 순서: 계정 전체 한도, 이어서 Antigravity 모델군. 목록에 없는 모델군은 indexOf의 -1이 `>>> 0`으로 최댓값이 되어 수집 순서대로 뒤에 온다.
const groupOrder = [DEFAULT_GROUP_ID, "gemini", "gemini-4-argon", "claude-gpt"];
const groupRank = (id: string) => groupOrder.indexOf(id) >>> 0;

export function modelGroups(quota: PlanQuota): ModelGroup[] {
  const groups = new Map<string, ModelGroup>();
  for (const window of quota.windows) {
    const id = groupId(window);
    const previous = groups.get(id);
    groups.set(id, { id, label: window.modelGroupLabel || quota.planName, windows: [...(previous?.windows ?? []), window] });
  }
  const sorted = [...groups.values()].sort((a, b) => groupRank(a.id) - groupRank(b.id));
  if (sorted.length < 2) return sorted;
  // Claude의 5시간(세션) 한도는 범위가 전체 모델이라 Fable 같은 모델별 사용에도 그대로 적용된다.
  // 서버가 모델별 5시간 한도를 따로 주지 않는 동안에는 그 창을 모델군에도 공용으로 표시해 함께 보여 준다.
  const sharedRolling = quota.providerId === "claude"
    ? groups.get(DEFAULT_GROUP_ID)?.windows.find(window => window.id === "rolling") : undefined;
  return sorted.map(group => {
    // 요금제 이름은 모델군 하나일 때만 모델군 이름을 겸한다.
    if (group.id === DEFAULT_GROUP_ID) return { ...group, label: ALL_MODELS_LABEL };
    if (!sharedRolling || group.windows.some(window => window.id === "rolling")) return group;
    return { ...group, windows: [{ ...sharedRolling, accountWide: true }, ...group.windows] };
  });
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
  // 계정 전체 한도가 있으면 그것이 기본이다. 모델별 한도는 사용자가 고를 때만 앞에 나온다.
  return groups.find(group => group.id === selected)
    ?? groups.find(group => group.id === DEFAULT_GROUP_ID)
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
