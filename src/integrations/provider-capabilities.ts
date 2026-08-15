import type { ProviderId } from "../data/providers";

export type OAuthCapability = "documented" | "api-key" | "provider-delegated" | "unverified";
export type QuotaScope = "subscription" | "api-project" | "organization" | "unverified";

export type ProviderCapability = Readonly<{
  providerId: ProviderId;
  oauth: OAuthCapability;
  oauthLabel: string;
  quotaScope: QuotaScope;
  quotaLabel: string;
  planQuotaVerified: boolean;
  docs: readonly string[];
  reviewedAt: string;
}>;

/**
 * Capability facts are deliberately separate from the example snapshot.
 * A provider can document OAuth for API access without exposing personal-plan
 * remaining quota. The UI must not promote that path to `confidence=verified`.
 */
export const providerCapabilities: Readonly<Record<ProviderId, ProviderCapability>> = Object.freeze({
  openai: {
    providerId: "openai",
    oauth: "api-key",
    oauthLabel: "OpenAI API 키 인증",
    quotaScope: "subscription",
    quotaLabel: "개인 요금제 잔여량 endpoint 미확인",
    planQuotaVerified: false,
    docs: Object.freeze([
      "https://platform.openai.com/docs/api-reference/authentication",
      "https://platform.openai.com/docs/api-reference/usage"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  claude: {
    providerId: "claude",
    oauth: "api-key",
    oauthLabel: "Anthropic API 키 인증",
    quotaScope: "subscription",
    quotaLabel: "개인 요금제 잔여량 endpoint 미확인",
    planQuotaVerified: false,
    docs: Object.freeze([
      "https://support.anthropic.com/en/articles/8114521-how-can-i-access-the-anthropic-api",
      "https://docs.anthropic.com/en/api/admin-api/usage-cost/get-messages-usage-report"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  gemini: {
    providerId: "gemini",
    oauth: "documented",
    oauthLabel: "Google OAuth · Gemini API",
    quotaScope: "api-project",
    quotaLabel: "Google Cloud 프로젝트 쿼터",
    planQuotaVerified: false,
    docs: Object.freeze([
      "https://ai.google.dev/gemini-api/docs/oauth",
      "https://ai.google.dev/gemini-api/docs/rate-limits"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  cursor: {
    providerId: "cursor",
    oauth: "unverified",
    oauthLabel: "공식 OAuth 경로 확인 필요",
    quotaScope: "unverified",
    quotaLabel: "공식 잔여량 범위 확인 필요",
    planQuotaVerified: false,
    docs: Object.freeze([] as const),
    reviewedAt: "2026-08-16"
  },
  copilot: {
    providerId: "copilot",
    oauth: "provider-delegated",
    oauthLabel: "Microsoft Entra delegated auth",
    quotaScope: "organization",
    quotaLabel: "Microsoft Graph/Copilot API 권한 범위",
    planQuotaVerified: false,
    docs: Object.freeze([
      "https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/copilot-apis-security-authentication",
      "https://learn.microsoft.com/en-us/graph/auth-v2-user"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  perplexity: {
    providerId: "perplexity",
    oauth: "unverified",
    oauthLabel: "공식 OAuth 경로 확인 필요",
    quotaScope: "unverified",
    quotaLabel: "공식 잔여량 범위 확인 필요",
    planQuotaVerified: false,
    docs: Object.freeze([] as const),
    reviewedAt: "2026-08-16"
  }
});
