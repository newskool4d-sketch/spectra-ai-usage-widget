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
  state: string;
  status: "native-boundary-ready";
}>;

export type NativeOAuthEvent = Readonly<{
  providerId: string;
  status: "callback-accepted";
}>;

export type NativeCredentialStatus = Readonly<{
  providerId: ProviderId;
  available: boolean;
  present: boolean;
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

export async function listenNativeOAuth(
  onEvent: (event: NativeOAuthEvent) => void,
): Promise<TauriUnlisten | null> {
  const listener = nativeGlobal()?.event?.listen;
  if (!listener) return null;
  return listener<NativeOAuthEvent>("oauth-callback", event => onEvent(event.payload));
}
