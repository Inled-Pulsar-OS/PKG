import { invoke } from "@tauri-apps/api/core";
import type { SystemMode } from "@/modules/core/types";

export function getSystemMode(): Promise<SystemMode> {
  return invoke("get_system_mode");
}

export function launchApp(app: string, fallback?: string): Promise<void> {
  return invoke("launch_app", { app, fallback: fallback ?? null });
}

export function isLiveSystem(): Promise<boolean> {
  return invoke("is_live_system");
}

export function isArchSystem(): Promise<boolean> {
  return invoke("is_arch_system");
}

export function checkSentinel(): Promise<boolean> {
  return invoke("check_sentinel");
}

export function writeSentinel(): Promise<void> {
  return invoke("write_sentinel");
}

export function getResolutions(): Promise<
  { width: number; height: number; active: boolean }[]
> {
  return invoke("get_resolutions");
}

export function launchDisplaySettings(): Promise<void> {
  return invoke("launch_display_settings");
}

export function launchWifiSettings(): Promise<void> {
  return invoke("launch_wifi_settings");
}

export function launchBluetoothSettings(): Promise<void> {
  return invoke("launch_bluetooth_settings");
}

export function getEffectsState(): Promise<boolean> {
  return invoke("get_effects_state");
}

export function setEffects(useLiquidGlass: boolean): Promise<void> {
  return invoke("set_effects", { useLiquidGlass });
}

export function checkAdbDevices(): Promise<string> {
  return invoke("check_adb_devices");
}

export function runCleanup(): Promise<void> {
  return invoke("run_cleanup");
}
