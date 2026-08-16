import type { ProviderId } from "../data/providers";

type TauriUnlisten = () => void | Promise<void>;
type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type TauriEvent<T> = Readonly<{ payload: T }>;
type TauriGlobal = Readonly<{
  core?: Readonly<{ invoke?: TauriInvoke }>;
  event?: Readonly<{
    listen?: <T>(event: string, handler: (event: TauriEvent<T>) => void) => Promise<TauriUnlisten>;
  }>;
}>;

declare global {
  interface Window {
    __TAURI__?: TauriGlobal;
  }
}

export type NativeOAuthPrepareResult = Readonly<{
  providerId: ProviderId;
  redirectUri: string;
  state: string | null;
  status: "authorize-ready" | "client-id-required" | "api-key-only" | "not-published";
  callbackMode: "loopback" | "custom-scheme" | "none";
  authorizeUrl: string | null;
  tokenUrl: string | null;
  clientIdEnv: string | null;
  clientIdConfigured: boolean;
  tokenExchangeStatus: "pkce-native" | "not-published";
  scope: string;
  quotaEndpoint: string | null;
  quotaEndpointStatus: "remaining-published" | "limit-metadata-only" | "organization-usage-only" | "api-key-only" | "not-published";
}>;

export type NativeOAuthEvent = Readonly<{
  providerId: string;
  status: "callback-accepted" | "credential-stored";
}>;

export type NativeOAuthRejected = Readonly<{
  reason: string;
}>;

export type NativeCredentialStatus = Readonly<{
  providerId: ProviderId;
  available: boolean;
  present: boolean;
}>;

export type NativeProviderQuotaWindow = Readonly<{
  id: "rolling" | "weekly";
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetsAt: number | null;
  windowDurationMins: number | null;
}>;

export type NativeProviderUsageSnapshot = Readonly<{
  providerId: ProviderId;
  runtimeAvailable: boolean;
  authState: "signed-in" | "signed-out" | "unknown";
  connectionState: "not-installed" | "signed-out" | "waiting-for-usage" | "connected" | "stale" | "error";
  authMethod: string | null;
  planType: string | null;
  source: "codex-app-server" | "claude-statusline" | null;
  lastSyncedAt: number | null;
  bridgeInstalled: boolean;
  windows: readonly NativeProviderQuotaWindow[];
  message: string;
}>;

export type NativeProviderActionResult = Readonly<{
  providerId: ProviderId;
  status: "login-started" | "bridge-installed" | "bridge-removed" | "not-installed" | "not-supported" | "not-available";
  message: string;
}>;

const nativeGlobal = (): TauriGlobal | undefined =>
  typeof window === "undefined" ? undefined : window.__TAURI__;

const invoke = (): TauriInvoke | undefined => nativeGlobal()?.core?.invoke;

export const isTauriRuntime = () => typeof invoke() === "function";

export async function prepareNativeOAuth(providerId: ProviderId): Promise<NativeOAuthPrepareResult | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeOAuthPrepareResult>("oauth_prepare", { providerId });
}

export async function getNativeCredentialStatus(providerId: ProviderId): Promise<NativeCredentialStatus | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeCredentialStatus>("credential_status", { providerId });
}

export async function removeNativeCredential(providerId: ProviderId): Promise<boolean> {
  const command = invoke();
  if (!command) return false;
  await command<void>("credential_remove", { providerId });
  return true;
}

export async function getNativeProviderUsage(providerId: ProviderId): Promise<NativeProviderUsageSnapshot | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeProviderUsageSnapshot>("provider_usage_snapshot", { providerId });
}

export async function startNativeProviderLogin(providerId: ProviderId): Promise<NativeProviderActionResult | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeProviderActionResult>("provider_start_login", { providerId });
}

export async function installNativeProviderBridge(providerId: ProviderId): Promise<NativeProviderActionResult | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeProviderActionResult>("provider_install_bridge", { providerId });
}

export async function removeNativeProviderBridge(providerId: ProviderId): Promise<NativeProviderActionResult | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeProviderActionResult>("provider_remove_bridge", { providerId });
}

export async function listenNativeOAuth(
  onEvent: (event: NativeOAuthEvent) => void,
): Promise<TauriUnlisten | null> {
  const listener = nativeGlobal()?.event?.listen;
  if (!listener) return null;
  return listener<NativeOAuthEvent>("oauth-callback", event => onEvent(event.payload));
}

export async function listenNativeOAuthRejected(
  onEvent: (event: NativeOAuthRejected) => void,
): Promise<TauriUnlisten | null> {
  const listener = nativeGlobal()?.event?.listen;
  if (!listener) return null;
  return listener<NativeOAuthRejected>("oauth-callback-rejected", event => onEvent(event.payload));
}
