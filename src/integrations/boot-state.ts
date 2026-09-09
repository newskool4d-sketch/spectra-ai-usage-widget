import type { NativeProviderUsageSnapshot } from "./tauri-native-bridge";

export type NativeUiPrefs = Readonly<{ theme: "dark" | "light"; solid: boolean; standby: boolean }>;
export type NativeBootState = NativeUiPrefs & Readonly<{ mode: "mini" | "dashboard"; snapshots: readonly NativeProviderUsageSnapshot[] }>;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null;
const knownProviders = new Set(["codex", "claude"]);

function isSnapshot(value: unknown): value is NativeProviderUsageSnapshot {
  return isRecord(value) && typeof value.providerId === "string" && typeof value.connectionState === "string" && Array.isArray(value.windows);
}

export function parseBootState(raw: unknown): NativeBootState | null {
  if (!isRecord(raw)) return null;
  const { theme, solid, standby, mode, snapshots } = raw;
  if (theme !== "dark" && theme !== "light") return null;
  if (mode !== "mini" && mode !== "dashboard") return null;
  if (typeof solid !== "boolean" || typeof standby !== "boolean" || !Array.isArray(snapshots)) return null;
  return {
    theme,
    solid,
    standby,
    mode,
    snapshots: snapshots.filter(isSnapshot).filter(snapshot => knownProviders.has(snapshot.providerId))
  };
}
