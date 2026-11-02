import type { ProviderId } from "../data/providers";
import type { NativeProviderUsageSnapshot } from "./tauri-native-bridge";

/** Antigravity's first official statusline payload establishes auth and usage together. */
export function nextConnectionAction(providerId: ProviderId, snapshot: Pick<NativeProviderUsageSnapshot, "runtimeAvailable" | "authState" | "bridgeInstalled">) {
  if (!snapshot.runtimeAvailable) return "install-cli";
  if (providerId === "antigravity" && !snapshot.bridgeInstalled) return "install-bridge";
  if (snapshot.authState === "signed-out" || (providerId !== "antigravity" && snapshot.authState !== "signed-in")) return "login";
  if (providerId === "claude" && !snapshot.bridgeInstalled) return "install-bridge";
  return "refresh";
}
