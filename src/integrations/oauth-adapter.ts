import type { ProviderId } from "../data/providers";
import { providerCapabilities, type ProviderCapability } from "./provider-capabilities";
import { isTauriRuntime, prepareNativeOAuth, removeNativeCredential } from "./tauri-native-bridge";

export type OAuthStartResult = Readonly<{
  status: "demo-only" | "native-ready" | "not-available";
  message: string;
  redirectUri?: string;
  state?: string;
}>;

/**
 * Native shells implement this boundary with an OS callback and a
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

const createStart = (providerId: ProviderId) => async (): Promise<OAuthStartResult> => {
  if (!isTauriRuntime()) return demoStart();

  try {
    const result = await prepareNativeOAuth(providerId);
    if (!result) return demoStart();
    return {
      status: "native-ready",
      message: "네이티브 callback과 OS 보관 경계가 준비되었습니다. 공급자별 authorize/token 교환 설정이 남아 있습니다.",
      redirectUri: result.redirectUri,
      state: result.state
    };
  } catch {
    return {
      status: "not-available",
      message: "이 기기에서 네이티브 OAuth 준비를 완료하지 못했습니다."
    };
  }
};

const createDisconnect = (providerId: ProviderId) => async () => {
  if (isTauriRuntime()) await removeNativeCredential(providerId);
};

const createDemoAdapter = (providerId: ProviderId): OAuthAdapter => Object.freeze({
  providerId,
  capability: providerCapabilities[providerId],
  start: createStart(providerId),
  disconnect: createDisconnect(providerId)
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
