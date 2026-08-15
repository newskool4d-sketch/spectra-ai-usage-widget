import type { ProviderId } from "../data/providers";
import { providerCapabilities, type ProviderCapability } from "./provider-capabilities";
import { isTauriRuntime, prepareNativeOAuth, removeNativeCredential } from "./tauri-native-bridge";

export type OAuthStartResult = Readonly<{
  status: "demo-only" | "authorize-ready" | "configuration-required" | "not-supported" | "not-available";
  message: string;
  redirectUri?: string;
  state?: string;
  authorizeUrl?: string;
  tokenUrl?: string;
  clientIdEnv?: string;
  quotaEndpoint?: string;
  quotaEndpointStatus?: string;
}>;

/**
 * The browser build remains demo-only. Native shells build the official
 * provider URL, open it in the system browser, exchange the callback code in
 * Rust/Swift, and write the resulting credential to the OS vault. Claude and
 * Codex currently stay on the official API-key/usage-page boundary because
 * personal subscription OAuth and remaining-quota endpoints are not public.
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

    if (result.status === "client-id-required") {
      return {
        status: "configuration-required",
        message: `${result.clientIdEnv ?? "공급자 client ID"} 환경 변수를 네이티브 앱에 설정해야 공식 로그인 창을 열 수 있습니다.`,
        redirectUri: result.redirectUri,
        clientIdEnv: result.clientIdEnv ?? undefined,
        quotaEndpoint: result.quotaEndpoint ?? undefined,
        quotaEndpointStatus: result.quotaEndpointStatus
      };
    }

    return {
      status: "not-supported",
      message: "Claude와 Codex는 개인 요금제용 공식 OAuth/잔여량 API가 공개되지 않아 비공개 세션 연결을 하지 않습니다. 공식 사용량 페이지 또는 조직 Admin API 범위만 사용합니다.",
      redirectUri: result.redirectUri,
      quotaEndpoint: result.quotaEndpoint ?? undefined,
      quotaEndpointStatus: result.quotaEndpointStatus
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
  codex: createDemoAdapter("codex"),
  claude: createDemoAdapter("claude")
});

export const getOAuthAdapter = (providerId: ProviderId) => oauthAdapters[providerId];
