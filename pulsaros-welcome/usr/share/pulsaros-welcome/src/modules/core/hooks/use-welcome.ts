import { useState, useCallback } from "react";
import type { WelcomeScreen, DataState } from "@/modules/core/types";
import {
  isLiveSystem,
  isArchSystem,
  isOotbPending,
  getResolutions,
  getEffectsState,
  wifiSlideEnabled,
  setEffects as apiSetEffects,
  launchOotb,
  writeSentinel,
  closeWindow,
} from "@/modules/core/api";

const ALL_STEPS: WelcomeScreen[] = [
  "hello",
  "features",
  "compatibility",
  "settings",
  "wifi",
  "sayri",
];

/**
 * The Wi-Fi configurator slide is DISABLED by default. Re-enable it by
 * launching the welcome app with PULSAROS_ENABLE_WIFI_SLIDE=1 — the env var is
 * read by the Python/WebKitGTK backend (welcome.py) and exposed through the
 * `wifi_slide_enabled` command on the Tauri backend.
 */
function buildFlow(wifiEnabled: boolean, isLive: boolean): WelcomeScreen[] {
  const steps = wifiEnabled ? ALL_STEPS : ALL_STEPS.filter((s) => s !== "wifi");
  const flow = [...steps];
  if (isLive) {
    flow.push("recovery");
  }
  flow.push("done");
  return flow;
}

export function useWelcome() {
  const [screen, setScreen] = useState<WelcomeScreen>("hello");
  const [{
    effectsState,
    isArch,
    isLive,
    ootbPending,
    resolutions,
    wifiSlide
  }, setData] = useState<DataState>({
    isLive: false,
    isArch: false,
    ootbPending: false,
    resolutions: [],
    effectsState: false,
    wifiSlide: false,
  });

  const loadSystemInfo = useCallback(async () => {
    const [live, arch, ootb, res, effects, wifi] = await Promise.all([
      isLiveSystem(),
      isArchSystem(),
      isOotbPending(),
      getResolutions(),
      getEffectsState(),
      wifiSlideEnabled(),
    ]);
    setData((prev) => ({
      ...prev,
      isLive: live,
      isArch: arch,
      ootbPending: ootb,
      resolutions: res,
      effectsState: effects,
      wifiSlide: wifi,
    }));
  }, []);

  const proceedFromHello = useCallback(async () => {
    const isPending = await isOotbPending();
    if (isPending || ootbPending) {
      await launchOotb();
      await closeWindow();
      return;
    }
    setScreen("features");
  }, [ootbPending]);

  const goNext = useCallback(() => {
    setScreen((prev) => {
      const flow = buildFlow(wifiSlide, isLive);
      const idx = flow.indexOf(prev);
      if (idx !== -1 && idx < flow.length - 1) {
        return flow[idx + 1];
      }
      return "done";
    });
  }, [isLive, wifiSlide]);

  const goBack = useCallback(() => {
    setScreen((prev) => {
      const flow = buildFlow(wifiSlide, isLive);
      const idx = flow.indexOf(prev);
      if (idx > 0) {
        return flow[idx - 1];
      }
      return prev;
    });
  }, [isLive, wifiSlide]);

  const goTo = useCallback((s: WelcomeScreen) => setScreen(s), []);

  const setEffectsValue = useCallback(async (useLiquidGlass: boolean) => {
    setData((prev) => ({ ...prev, effectsState: useLiquidGlass }));
    await apiSetEffects(useLiquidGlass);
  }, []);

  const complete = useCallback(async () => {
    await writeSentinel();
    setScreen("done");
  }, []);

  const restart = useCallback(() => {
    setScreen("hello");
  }, []);

  return {
    screen,
    isLive,
    isArch,
    ootbPending,
    resolutions,
    effectsState,
    loadSystemInfo,
    proceedFromHello,
    goNext,
    goBack,
    goTo,
    setEffects: setEffectsValue,
    complete,
    restart,
  };
}