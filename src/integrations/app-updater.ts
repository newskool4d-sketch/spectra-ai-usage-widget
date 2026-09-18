import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { isTauriRuntime } from "./tauri-native-bridge";

export type AvailableAppUpdate = Readonly<{
  version: string;
  notes: string;
  install: () => Promise<void>;
}>;

export async function checkForAppUpdate(): Promise<AvailableAppUpdate | null> {
  if (!isTauriRuntime()) return null;
  const update = await check();
  if (!update) return null;

  return {
    version: update.version,
    notes: update.body ?? "새 버전이 준비되었습니다.",
    install: async () => {
      await update.downloadAndInstall();
      await relaunch();
    },
  };
}
