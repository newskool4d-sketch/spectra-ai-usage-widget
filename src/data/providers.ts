export type ProviderId = "codex" | "claude";
export type Metric = "remaining" | "cost" | "requests";
export type UsageRange = "24H" | "7D" | "30D";
export type QuotaWindowId = "rolling" | "weekly";
export type QuotaConnectionState =
  | "not_connected"
  | "not_installed"
  | "signed_out"
  | "waiting"
  | "connected"
  | "stale"
  | "unsupported"
  | "error";
export type QuotaSource = "example" | "codex-app-server" | "claude-statusline" | "unavailable";
export type QuotaConfidence = "example" | "verified" | "unavailable";
export type AuthMethod = "oauth-pkce" | "provider-delegated" | "not-published";

export type QuotaWindow = Readonly<{
  id: QuotaWindowId;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetLabel: string;
  kindLabel: string;
}>;

export type PlanQuota = Readonly<{
  providerId: ProviderId;
  planName: string;
  accountLabel: string;
  authMethod: AuthMethod;
  connectionState: QuotaConnectionState;
  source: QuotaSource;
  confidence: QuotaConfidence;
  lastSyncedAt: string | null;
  bridgeInstalled: boolean;
  statusMessage: string;
  windows: readonly QuotaWindow[];
}>;

export type Provider = Readonly<{
  id: ProviderId;
  name: string;
  short: string;
  color: string;
  used: number;
  tokens: string;
  cost: string;
  reset: string;
  trend: readonly number[];
}>;

export const providers: readonly Provider[] = Object.freeze([
  { id: "codex", name: "Codex", short: "CX", color: "#56B7B0", used: 68, tokens: "3.8M", cost: "$18.40", reset: "3시간 42분", trend: [28, 34, 31, 48, 44, 63, 68] },
  { id: "claude", name: "Claude", short: "CL", color: "#D88463", used: 76, tokens: "2.6M", cost: "$14.90", reset: "1시간 18분", trend: [40, 42, 51, 49, 64, 71, 76] }
]);

export const planQuotas: Readonly<Record<ProviderId, PlanQuota>> = Object.freeze({
  codex: {
    providerId: "codex",
    planName: "ChatGPT Pro · Codex",
    accountLabel: "브라우저 데모 · 네이티브 앱에서 연결",
    authMethod: "provider-delegated",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    bridgeInstalled: false,
    statusMessage: "네이티브 앱에서는 Codex App Server로 실제 한도를 확인합니다.",
    windows: Object.freeze([
      { id: "rolling", label: "에이전트 한도", usedPercent: 76, remainingPercent: 24, resetLabel: "1시간 18분 후", kindLabel: "공유 에이전트 사용량" },
      { id: "weekly", label: "주간 에이전트 한도", usedPercent: 42, remainingPercent: 58, resetLabel: "3일 4시간 후", kindLabel: "주간 사용량" }
    ] as const)
  },
  claude: {
    providerId: "claude",
    planName: "Claude Max",
    accountLabel: "브라우저 데모 · 네이티브 앱에서 연결",
    authMethod: "provider-delegated",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    bridgeInstalled: false,
    statusMessage: "네이티브 앱에서는 Claude Code 상태선 브리지로 실제 한도를 확인합니다.",
    windows: Object.freeze([
      { id: "rolling", label: "5시간 한도", usedPercent: 68, remainingPercent: 32, resetLabel: "3시간 42분 후", kindLabel: "공유 사용량" },
      { id: "weekly", label: "주간 한도", usedPercent: 51, remainingPercent: 49, resetLabel: "4일 2시간 후", kindLabel: "주간 사용량" }
    ] as const)
  }
});

export const usageBars = Object.freeze([43, 58, 49, 70, 64, 83, 74, 91, 66, 79, 88, 72, 96, 82]);

export const metricLabels: Readonly<Record<Metric, string>> = Object.freeze({
  remaining: "잔여량",
  cost: "비용",
  requests: "요청"
});

export const rangeLabels: Readonly<Record<UsageRange, string>> = Object.freeze({
  "24H": "오늘",
  "7D": "7일",
  "30D": "30일"
});
