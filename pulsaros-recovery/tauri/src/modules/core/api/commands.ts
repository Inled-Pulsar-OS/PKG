import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BtrfsTarget, SystemMode } from "@/modules/core/types";

export function getSystemMode(): Promise<SystemMode> {
  return invoke("get_system_mode");
}

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

// ── Installer ──

export function startInstall(
  diskPath: string,
  installBroadcom: boolean
): Promise<void> {
  return invoke("start_install", { diskPath, installBroadcom });
}

export function detectBroadcom(): Promise<boolean> {
  return invoke("detect_broadcom");
}

export function onInstallProgress(
  cb: (progress: number, status: string) => void
): Promise<() => void> {
  return listen<{ progress: number; status: string }>(
    "install-progress",
    (e) => cb(e.payload.progress, e.payload.status)
  );
}

export function onInstallLog(cb: (message: string) => void): Promise<() => void> {
  return listen<{ message: string }>("install-log", (e) =>
    cb(e.payload.message)
  );
}

// ── OOTB ──

export function startOotbSetup(params: {
  fullname: string;
  username: string;
  password: string;
  language: string;
  keymap: string;
  timezone: string;
  avatarPath?: string;
}): Promise<void> {
  return invoke("start_ootb_setup", {
    fullname: params.fullname,
    username: params.username,
    password: params.password,
    language: params.language,
    keymap: params.keymap,
    timezone: params.timezone,
    avatarPath: params.avatarPath ?? null,
  });
}

export function ootbFinalCleanup(username: string): Promise<void> {
  return invoke("ootb_final_cleanup", { username });
}

export function onOotbProgress(
  cb: (progress: number, status: string) => void
): Promise<() => void> {
  return listen<{ progress: number; status: string }>(
    "ootb-progress",
    (e) => cb(e.payload.progress, e.payload.status)
  );
}

export function onOotbLog(cb: (message: string) => void): Promise<() => void> {
  return listen<{ message: string }>("ootb-log", (e) =>
    cb(e.payload.message)
  );
}
