import type { SystemMode } from "@/modules/core/types";
import type { WifiNetwork } from "@/modules/wifi/types";

declare global {
  interface Window {
    webkit?: {
      messageHandlers?: {
        welcome?: {
          postMessage: (msg: string) => void;
        };
      };
    };
    __PY_RESPONSES__?: Record<string, (val: any) => void>;
    __handlePyResponse?: (id: string, val: any) => void;
    IS_LIVE?: boolean;
    IS_ARCH?: boolean;
    IS_OOTB?: boolean;
  }
}

if (typeof window !== "undefined") {
  window.__handlePyResponse = (id: string, val: any) => {
    if (window.__PY_RESPONSES__ && window.__PY_RESPONSES__[id]) {
      window.__PY_RESPONSES__[id](val);
      delete window.__PY_RESPONSES__[id];
    }
  };
}

let reqId = 0;
export async function invoke<T = any>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  // If running inside Tauri
  if (typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__) {
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    return tauriInvoke<T>(cmd, args);
  }

  // If running inside Python WebKitGTK
  if (typeof window !== "undefined" && window.webkit?.messageHandlers?.welcome) {
    return new Promise((resolve) => {
      const id = String(++reqId);
      if (!window.__PY_RESPONSES__) window.__PY_RESPONSES__ = {};
      window.__PY_RESPONSES__[id] = resolve;
      window.webkit!.messageHandlers!.welcome!.postMessage(
        JSON.stringify({ cmd, args, id })
      );
    });
  }

  // Environment variable fallbacks for direct browser/standalone testing
  if (cmd === "is_live_system") return (window.IS_LIVE ?? false) as any;
  if (cmd === "is_arch_system") return (window.IS_ARCH ?? true) as any;
  if (cmd === "is_ootb_pending") return (window.IS_OOTB ?? false) as any;
  if (cmd === "check_sentinel") return false as any;
  if (cmd === "get_system_mode") return "Normal" as any;
  if (cmd === "get_resolutions") return [] as any;
  if (cmd === "get_effects_state") return false as any;
  if (cmd === "check_adb_devices") return "" as any;
  if (cmd === "scan_wifi_networks") return [] as any;
  if (cmd === "connect_to_wifi") return false as any;

  return undefined as any;
}

export function closeWindow(): Promise<void> {
  return invoke("close");
}

export function openUrl(url: string): Promise<void> {
  return invoke("open_url", { url });
}

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

export function isOotbPending(): Promise<boolean> {
  return invoke("is_ootb_pending");
}

export function launchOotb(): Promise<void> {
  return invoke("launch_ootb");
}

export function launchRecovery(): Promise<void> {
  return invoke("launch_recovery");
}

export function writeSentinel(): Promise<void> {
  return invoke("write_sentinel");
}

export function getResolutions(): Promise<
  { width: number; height: number; active: boolean }[]
> {
  return invoke("get_resolutions");
}

export function setResolution(width: number, height: number): Promise<void> {
  return invoke("set_resolution", { width, height });
}

export function launchDisplaySettings(): Promise<void> {
  return invoke("launch_display_settings");
}

export function launchAppearanceSettings(): Promise<void> {
  return invoke("launch_appearance_settings");
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

export function scanWifiNetworks(): Promise<WifiNetwork[]> {
  return invoke("scan_wifi_networks");
}

export function connectToWifi(ssid: string, password?: string): Promise<boolean> {
  return invoke("connect_to_wifi", { ssid, password: password ?? null });
}
