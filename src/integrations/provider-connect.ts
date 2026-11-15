import type { ProviderId } from "../data/providers";
import type { NativeProviderUsageSnapshot } from "./tauri-native-bridge";

export type ConnectionAction = "login" | "install-bridge" | "refresh";

export function nextConnectionAction(
  providerId: ProviderId,
  snapshot: Pick<NativeProviderUsageSnapshot, "runtimeAvailable" | "authState" | "bridgeInstalled">,
  requested: ConnectionAction
) {
  if (!snapshot.runtimeAvailable) return "install-cli";
  // Antigravity establishes auth and usage together on its first statusline payload.
  const canInstallBridge = !snapshot.bridgeInstalled
    && (providerId === "antigravity" || (providerId === "claude" && snapshot.authState === "signed-in"));
  if (requested === "install-bridge" && canInstallBridge) return "install-bridge";
  if (requested === "login" && !canInstallBridge
    && (snapshot.authState === "signed-out" || (providerId !== "antigravity" && snapshot.authState !== "signed-in"))) return "login";
  return "refresh";
}
