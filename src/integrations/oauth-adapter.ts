import type { ProviderId } from "../data/providers";
import { providerCapabilities, type ProviderCapability } from "./provider-capabilities";

export type OAuthStartResult = Readonly<{
  status: "demo-only" | "not-available";
  message: string;
}>;

/**
 * Native shells will implement this boundary with an OS callback and a
 * CredentialVault. The web prototype intentionally has no network or token
 * exchange implementation.
 */
export type OAuthAdapter = Readonly<{
  providerId: ProviderId;
  capability: ProviderCapability;
  start: () => Promise<OAuthStartResult>;
  disconnect: () => Promise<void>;
}>;

const demoStart = async (): Promise<OAuthStartResult> => ({
  status: "demo-only",
  message: "실제 공급자 호출 전 제품 흐름만 확인하는 데모입니다."
});

const demoDisconnect = async () => undefined;

const createDemoAdapter = (providerId: ProviderId): OAuthAdapter => Object.freeze({
  providerId,
  capability: providerCapabilities[providerId],
  start: demoStart,
  disconnect: demoDisconnect
});

export const oauthAdapters: Readonly<Record<ProviderId, OAuthAdapter>> = Object.freeze({
  openai: createDemoAdapter("openai"),
  claude: createDemoAdapter("claude"),
  gemini: createDemoAdapter("gemini"),
  cursor: createDemoAdapter("cursor"),
  copilot: createDemoAdapter("copilot"),
  perplexity: createDemoAdapter("perplexity")
});

export const getOAuthAdapter = (providerId: ProviderId) => oauthAdapters[providerId];
