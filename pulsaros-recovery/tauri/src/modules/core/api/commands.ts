import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BtrfsTarget } from "@/modules/core/types";

export function getBtrfsTargets(): Promise<BtrfsTarget[]> {
  return invoke("get_btrfs_targets");
}

export function getLocalSquashfs(): Promise<string | null> {
  return invoke("get_local_squashfs");
}

export function startRestore(
  target: BtrfsTarget,
  internetUrl?: string
): Promise<void> {
  return invoke("start_restore", {
    target,
    internetUrl: internetUrl ?? null,
  });
}

export function launchApp(app: string): Promise<void> {
  return invoke("launch_app", { app });
}

export function reboot(): Promise<void> {
  return invoke("reboot");
}

export function onRestoreProgress(
  cb: (progress: number, status: string) => void
): Promise<() => void> {
  return listen<{ progress: number; status: string }>(
    "restore-progress",
    (e) => cb(e.payload.progress, e.payload.status)
  );
}

export function onRestoreLog(cb: (message: string) => void): Promise<() => void> {
  return listen<{ message: string }>("restore-log", (e) =>
    cb(e.payload.message)
  );
}
