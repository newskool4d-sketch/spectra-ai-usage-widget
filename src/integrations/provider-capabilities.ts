import type { ProviderId } from "../data/providers";

export type OAuthCapability = "documented" | "api-key" | "app-server" | "cli-managed" | "provider-delegated" | "unverified";
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
 * Native integrations use only provider-owned, documented account surfaces.
 * SPECTRA never copies provider tokens into React or browser storage.
 */
export const providerCapabilities: Readonly<Record<ProviderId, ProviderCapability>> = Object.freeze({
  codex: {
    providerId: "codex",
    oauth: "app-server",
    oauthLabel: "Codex App Server · ChatGPT 로그인",
    quotaScope: "subscription",
    quotaLabel: "ChatGPT 요금제 한도 창·사용률·초기화 시각",
    planQuotaVerified: true,
    authorizeUrl: null,
    tokenUrl: null,
    clientIdEnv: null,
    connectionStatus: "supported",
    quotaEndpoint: "account/rateLimits/read",
    quotaEndpointStatus: "remaining-published",
    docs: Object.freeze([
      "https://developers.openai.com/codex/auth",
      "https://developers.openai.com/codex/app-server"
    ] as const),
    reviewedAt: "2026-08-16"
  },
  claude: {
    providerId: "claude",
    oauth: "cli-managed",
    oauthLabel: "Claude Code · Claude.ai 로그인",
    quotaScope: "subscription",
    quotaLabel: "Pro/Max 5시간·7일 사용률·초기화 시각",
    planQuotaVerified: true,
    authorizeUrl: null,
    tokenUrl: null,
    clientIdEnv: null,
    connectionStatus: "supported",
    quotaEndpoint: "Claude Code statusLine.rate_limits",
    quotaEndpointStatus: "remaining-published",
    docs: Object.freeze([
      "https://code.claude.com/docs/en/cli-usage",
      "https://code.claude.com/docs/en/statusline",
      "https://support.claude.com/en/articles/11145838-use-claude-code-with-your-pro-or-max-plan"
    ] as const),
    reviewedAt: "2026-08-16"
  }
});
