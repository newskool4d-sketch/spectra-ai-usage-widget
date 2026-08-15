export type ProviderId = "openai" | "claude" | "gemini" | "cursor" | "copilot" | "perplexity";
export type Metric = "remaining" | "cost" | "requests";
export type UsageRange = "24H" | "7D" | "30D";
export type QuotaWindowId = "rolling" | "weekly";
export type QuotaConnectionState = "not_connected" | "connected" | "unsupported" | "error";
export type QuotaSource = "example" | "oauth" | "unavailable";
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
  { id: "openai", name: "OpenAI", short: "OA", color: "#56B7B0", used: 68, tokens: "3.8M", cost: "$18.40", reset: "3시간 42분", trend: [28, 34, 31, 48, 44, 63, 68] },
  { id: "claude", name: "Claude", short: "CL", color: "#D88463", used: 76, tokens: "2.6M", cost: "$14.90", reset: "1시간 18분", trend: [40, 42, 51, 49, 64, 71, 76] },
  { id: "gemini", name: "Gemini", short: "GM", color: "#8879C6", used: 41, tokens: "1.2M", cost: "$6.20", reset: "8시간", trend: [22, 33, 28, 37, 44, 39, 41] },
  { id: "cursor", name: "Cursor", short: "CU", color: "#8CB66B", used: 54, tokens: "892K", cost: "$4.80", reset: "12일", trend: [18, 26, 31, 38, 36, 49, 54] },
  { id: "copilot", name: "Copilot", short: "CP", color: "#C77799", used: 29, tokens: "440K", cost: "$2.10", reset: "18일", trend: [12, 18, 21, 17, 26, 24, 29] },
  { id: "perplexity", name: "Perplexity", short: "PX", color: "#C7A852", used: 18, tokens: "318K", cost: "$1.70", reset: "18일", trend: [9, 12, 15, 11, 16, 14, 18] }
]);

export const planQuotas: Readonly<Record<ProviderId, PlanQuota>> = Object.freeze({
  openai: {
    providerId: "openai",
    planName: "ChatGPT Pro",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "not-published",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "5시간 한도", usedPercent: 76, remainingPercent: 24, resetLabel: "1시간 18분 후", kindLabel: "롤링 한도" },
      { id: "weekly", label: "주간 한도", usedPercent: 42, remainingPercent: 58, resetLabel: "3일 4시간 후", kindLabel: "주간 한도" }
    ] as const)
  },
  claude: {
    providerId: "claude",
    planName: "Claude Max",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "not-published",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "5시간 한도", usedPercent: 68, remainingPercent: 32, resetLabel: "3시간 42분 후", kindLabel: "롤링 한도" },
      { id: "weekly", label: "주간 한도", usedPercent: 51, remainingPercent: 49, resetLabel: "4일 2시간 후", kindLabel: "주간 한도" }
    ] as const)
  },
  gemini: {
    providerId: "gemini",
    planName: "Google AI Pro",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "oauth-pkce",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "일일 한도", usedPercent: 41, remainingPercent: 59, resetLabel: "8시간 후", kindLabel: "일일 한도" },
      { id: "weekly", label: "주간 한도", usedPercent: 27, remainingPercent: 73, resetLabel: "5일 후", kindLabel: "주간 한도" }
    ] as const)
  },
  cursor: {
    providerId: "cursor",
    planName: "Cursor Pro",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "not-published",
    connectionState: "unsupported",
    source: "unavailable",
    confidence: "unavailable",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "사용량 한도", usedPercent: 54, remainingPercent: 46, resetLabel: "12일 후", kindLabel: "월간 한도" },
      { id: "weekly", label: "보너스 요청", usedPercent: 29, remainingPercent: 71, resetLabel: "12일 후", kindLabel: "월간 한도" }
    ] as const)
  },
  copilot: {
    providerId: "copilot",
    planName: "Microsoft Copilot Pro",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "provider-delegated",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "월간 한도", usedPercent: 29, remainingPercent: 71, resetLabel: "18일 후", kindLabel: "월간 한도" },
      { id: "weekly", label: "우선 요청", usedPercent: 18, remainingPercent: 82, resetLabel: "18일 후", kindLabel: "월간 한도" }
    ] as const)
  },
  perplexity: {
    providerId: "perplexity",
    planName: "Perplexity Pro",
    accountLabel: "예시 계정 · OAuth 연결 전",
    authMethod: "not-published",
    connectionState: "not_connected",
    source: "example",
    confidence: "example",
    lastSyncedAt: null,
    windows: Object.freeze([
      { id: "rolling", label: "월간 한도", usedPercent: 18, remainingPercent: 82, resetLabel: "18일 후", kindLabel: "월간 한도" },
      { id: "weekly", label: "고급 요청", usedPercent: 11, remainingPercent: 89, resetLabel: "18일 후", kindLabel: "월간 한도" }
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
