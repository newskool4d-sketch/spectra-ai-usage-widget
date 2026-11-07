import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { isTauriRuntime, readBootState } from "./tauri-native-bridge";

export const macUpdateMessage = "Mac 시험판은 새 설치파일로 업데이트합니다. 자동 업데이트는 정식 배포 후 지원됩니다.";

// Native request deadlines cancel the network operation itself; a JS race would
// leave a timed-out download running and could later launch an unwanted install.
const manifestTimeoutMs = 15_000;
const downloadTimeoutMs = 120_000;

export type AvailableAppUpdate = Readonly<{
  version: string;
  notes: string;
  install: () => Promise<void>;
}>;

export async function checkForAppUpdate(): Promise<AvailableAppUpdate | null> {
  if (!isTauriRuntime()) return null;
  // Mac preview packages intentionally have no signed updater feed yet. Never query
  // the Windows release manifest or report a missing Mac entry as up to date.
  if (readBootState()?.platform === "macos") throw new Error(macUpdateMessage);
  const update = await check({ timeout: manifestTimeoutMs });
  if (!update) return null;

  let installation: Promise<void> | null = null;
  let installed = false;
  return {
    version: update.version,
    notes: update.body ?? "새 버전이 준비되었습니다.",
    install: () => {
      if (installation) return installation;
      installation = (async () => {
        if (!installed) {
          await update.downloadAndInstall(undefined, { timeout: downloadTimeoutMs });
          installed = true;
        }
        await relaunch();
      })().catch(error => {
        installation = null;
        throw error;
      });
      return installation;
    },
  };
}
