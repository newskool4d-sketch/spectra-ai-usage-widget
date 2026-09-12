import type { NativeProviderUsageSnapshot } from "./tauri-native-bridge";

export type NativeUiPrefs = Readonly<{ theme: "dark" | "light"; solid: boolean; standby: boolean; strip: boolean }>;
export type NativeBootState = NativeUiPrefs & Readonly<{ mode: "mini" | "dashboard"; snapshots: readonly NativeProviderUsageSnapshot[] }>;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null;
const knownProviders = new Set(["codex", "claude"]);

function isWindow(value: unknown): boolean {
  return isRecord(value) && typeof value.id === "string" && typeof value.label === "string" && typeof value.usedPercent === "number" && typeof value.remainingPercent === "number";
}

function isSnapshot(value: unknown): value is NativeProviderUsageSnapshot {
  return isRecord(value) && typeof value.providerId === "string" && typeof value.connectionState === "string" && Array.isArray(value.windows) && value.windows.every(isWindow);
}

export function parseBootState(raw: unknown): NativeBootState | null {
  if (!isRecord(raw)) return null;
  const { theme, solid, standby, strip, mode, snapshots } = raw;
  if (theme !== "dark" && theme !== "light") return null;
  if (mode !== "mini" && mode !== "dashboard") return null;
  if (typeof solid !== "boolean" || typeof standby !== "boolean" || !Array.isArray(snapshots)) return null;
  return {
    theme,
    solid,
    standby,
    strip: typeof strip === "boolean" ? strip : false,
    mode,
    snapshots: snapshots.filter(isSnapshot).filter(snapshot => knownProviders.has(snapshot.providerId))
  };
}

/** Providers whose snapshot was not restored from the boot payload and still need an initial refresh. */
export function providersMissingFromBoot<T extends string>(boot: NativeBootState | null, all: readonly T[]): T[] {
  if (!boot) return [...all];
  const restored = new Set<string>(boot.snapshots.map(snapshot => snapshot.providerId));
  return all.filter(id => !restored.has(id));
}
