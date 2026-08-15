export type ProviderId = "openai" | "claude" | "gemini" | "cursor" | "copilot" | "perplexity";
export type Metric = "tokens" | "cost" | "requests";
export type UsageRange = "24H" | "7D" | "30D";

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

export const usageBars = Object.freeze([43, 58, 49, 70, 64, 83, 74, 91, 66, 79, 88, 72, 96, 82]);

export const metricLabels: Readonly<Record<Metric, string>> = Object.freeze({
  tokens: "토큰",
  cost: "비용",
  requests: "요청"
});

export const rangeLabels: Readonly<Record<UsageRange, string>> = Object.freeze({
  "24H": "오늘",
  "7D": "7일",
  "30D": "30일"
});
