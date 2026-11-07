export type ProviderId = "codex" | "claude" | "antigravity";
export type Metric = "remaining" | "subscription" | "requests";
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
export type QuotaSource = "example" | "codex-app-server" | "claude-statusline" | "claude-usage-api" | "antigravity-statusline" | "unavailable";
export type QuotaConfidence = "example" | "verified" | "unavailable";
export type AuthMethod = "oauth-pkce" | "provider-delegated" | "not-published";

export type QuotaWindow = Readonly<{
  id: QuotaWindowId;
  modelGroupId?: string;
  modelGroupLabel?: string;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetLabel: string;
  kindLabel: string;
  resetsAt?: number | null; // epoch ms
  windowDurationMins?: number | null;
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
  lastSyncedAtMs?: number | null; // Data capture time, not the refresh request time.
  bridgeInstalled: boolean;
  statusMessage: string;
  liveFailure?: string | null; // Native lookup failure code; undefined for browser demo data.
  windows: readonly QuotaWindow[];
}>;

export type Provider = Readonly<{
  id: ProviderId;
  name: string;
  short: string;
  color: string;
}>;

export const providers: readonly Provider[] = Object.freeze([
  { id: "codex", name: "Codex", short: "CX", color: "var(--color-cyan)" },
  { id: "claude", name: "Claude", short: "CL", color: "var(--color-coral)" },
  { id: "antigravity", name: "Antigravity", short: "AG", color: "var(--color-violet)" }
]);

const exampleQuota = Object.freeze({
  accountLabel: "브라우저 데모 · 네이티브 앱에서 연결",
  authMethod: "provider-delegated",
  connectionState: "not_connected",
  source: "example",
  confidence: "example",
  lastSyncedAt: null,
  bridgeInstalled: false
} as const);

function exampleWindow(id: QuotaWindowId, remainingPercent: number, resetLabel: string, modelGroupId?: string, modelGroupLabel?: string): QuotaWindow {
  return { id, modelGroupId, modelGroupLabel, label: id === "rolling" ? "5시간 한도" : "주간 한도", usedPercent: 100 - remainingPercent, remainingPercent, resetLabel, kindLabel: "구독 한도" };
}

export const planQuotas: Readonly<Record<ProviderId, PlanQuota>> = Object.freeze({
  codex: {
    ...exampleQuota,
    providerId: "codex",
    planName: "ChatGPT Pro · Codex",
    statusMessage: "설치된 앱에서 Codex 한도를 확인합니다.",
    windows: Object.freeze([
      exampleWindow("rolling", 24, "1시간 18분 후"),
      exampleWindow("weekly", 58, "3일 4시간 후")
    ] as const)
  },
  claude: {
    ...exampleQuota,
    providerId: "claude",
    planName: "Claude Max",
    statusMessage: "설치된 앱에서 Claude 한도를 확인합니다.",
    windows: Object.freeze([
      exampleWindow("rolling", 32, "3시간 42분 후"),
      exampleWindow("weekly", 49, "4일 2시간 후")
    ] as const)
  },
  antigravity: {
    ...exampleQuota,
    providerId: "antigravity",
    planName: "Google AI Pro",
    statusMessage: "Antigravity 앱에서 확인한 모델군별 한도를 표시합니다.",
    windows: Object.freeze([
      exampleWindow("rolling", 100, "3시간 41분 후", "gemini", "Gemini"),
      exampleWindow("weekly", 100, "5일 10시간 후", "gemini", "Gemini"),
      exampleWindow("weekly", 83, "6일 4시간 후", "gemini-4-argon", "Gemini 4 Argon"),
      exampleWindow("rolling", 100, "초기화 시각 미제공", "claude-gpt", "Claude · GPT"),
      exampleWindow("weekly", 100, "초기화 시각 미제공", "claude-gpt", "Claude · GPT")
    ] as const)
  }
});

export const metricLabels: Readonly<Record<Metric, string>> = Object.freeze({
  remaining: "잔여량",
  subscription: "구독 모델",
  requests: "요청"
});

export const rangeLabels: Readonly<Record<UsageRange, string>> = Object.freeze({
  "24H": "오늘",
  "7D": "7일",
  "30D": "30일"
});
