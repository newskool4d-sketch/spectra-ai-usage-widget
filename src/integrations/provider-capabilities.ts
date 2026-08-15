import type { ProviderId } from "../data/providers";

export type OAuthCapability = "documented" | "api-key" | "provider-delegated" | "unverified";
export type QuotaScope = "subscription" | "api-project" | "organization" | "unverified";
export type ConnectionStatus = "supported" | "client-id-required" | "api-key-only" | "not-published";
export type QuotaEndpointStatus =
  | "remaining-published"
  | "limit-metadata-only"
  | "organization-usage-only"
  | "api-key-only"
  | "not-published";

export type ProviderCapability = Readonly<{
  providerId: ProviderId;
  oauth: OAuthCapability;
  oauthLabel: string;
  quotaScope: QuotaScope;
  quotaLabel: string;
  planQuotaVerified: boolean;
  authorizeUrl: string | null;
  tokenUrl: string | null;
  clientIdEnv: string | null;
  connectionStatus: ConnectionStatus;
  quotaEndpoint: string | null;
  quotaEndpointStatus: QuotaEndpointStatus;
  docs: readonly string[];
  reviewedAt: string;
}>;

/**
 * Capability facts are deliberately separate from the example snapshot.
 * A provider can document OAuth for API access without exposing personal-plan
 * remaining quota. The UI must not promote that path to `confidence=verified`.
 */
export const providerCapabilities: Readonly<Record<ProviderId, ProviderCapability>> = Object.freeze({
  codex: {
    providerId: "codex",
    oauth: "api-key",
    oauthLabel: "OpenAI 조직 Admin API 키",
    quotaScope: "organization",
    quotaLabel: "ChatGPT/Codex 개인 잔여량 조회 경로 미공개 · 조직 API 사용량만",
    planQuotaVerified: false,
    authorizeUrl: null,
    tokenUrl: null,
    clientIdEnv: null,
    connectionStatus: "api-key-only",
    quotaEndpoint: "https://api.openai.com/v1/organization/usage/completions",
    quotaEndpointStatus: "organization-usage-only",
    docs: Object.freeze([
      "https://platform.openai.com/docs/api-reference/authentication",
      "https://platform.openai.com/docs/api-reference/usage",
      "https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  claude: {
    providerId: "claude",
    oauth: "api-key",
    oauthLabel: "Anthropic Admin API 키",
    quotaScope: "organization",
    quotaLabel: "Claude Pro/Max 개인 잔여량 조회 경로 미공개 · 조직 사용량만",
    planQuotaVerified: false,
    authorizeUrl: null,
    tokenUrl: null,
    clientIdEnv: null,
    connectionStatus: "api-key-only",
    quotaEndpoint: "https://api.anthropic.com/v1/organizations/usage_report/messages",
    quotaEndpointStatus: "organization-usage-only",
    docs: Object.freeze([
      "https://support.anthropic.com/en/articles/8114521-how-can-i-access-the-anthropic-api",
      "https://docs.anthropic.com/en/api/admin-api/usage-cost/get-messages-usage-report",
      "https://support.anthropic.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan"
    ] as const),
    reviewedAt: "2026-08-16"
  }
});
