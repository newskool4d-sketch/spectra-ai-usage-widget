import type { PlanQuota } from "./providers.ts";

// Keep the labels and expiry boundary aligned with taskbar_strip::claude_status.
export function claudeFreshness(quota: PlanQuota, now: number): Readonly<{ label: string; tooltip: string }> {
  let label: string;
  const captured = quota.lastSyncedAtMs;
  const hasUsage = quota.confidence === "verified" && quota.windows.length > 0;
  const expired = quota.windows.some(window => window.resetsAt != null && window.resetsAt <= now);
  const capturedRecently = captured != null && Number.isFinite(captured) && captured <= now && now - captured <= 86_400_000;

  if (quota.source === "example") label = "브라우저 데모";
  else if (quota.connectionState === "not_installed") label = "Claude Code 설치 필요";
  else if (quota.connectionState === "signed_out") label = "로그인 필요";
  else if (quota.connectionState === "error" && !hasUsage) label = "연결 상태 확인 필요";
  else if (!hasUsage) label = "사용량 갱신 대기";
  else if (quota.connectionState !== "connected" || expired || !capturedRecently || quota.source !== "claude-usage-api") label = "갱신 대기 (캐시)";
  else label = "동기화됨";

  const synced = quota.source === "example" ? "데모 데이터" : quota.lastSyncedAt ?? "기록 없음";
  const lines = [`Claude · ${label}`, `마지막 동기화: ${synced} (데이터 수집 기준)`];
  if (quota.connectionState !== "connected" && quota.statusMessage) lines.push(quota.statusMessage);
  return { label, tooltip: lines.join("\n") };
}
