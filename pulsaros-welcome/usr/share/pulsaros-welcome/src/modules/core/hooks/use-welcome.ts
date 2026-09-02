import { useState, useCallback } from "react";
import type { WelcomeScreen, DataState } from "@/modules/core/types";
import {
  isLiveSystem,
  isArchSystem,
  isOotbPending,
  getResolutions,
  getEffectsState,
  setEffects as apiSetEffects,
  launchOotb,
  writeSentinel,
  closeWindow,
} from "@/modules/core/api";

const BASE_FLOW: WelcomeScreen[] = [
  "hello",
  "features",
  "compatibility",
  "settings",
  "wifi",
  "sayri",
];

export function useWelcome() {
  const [screen, setScreen] = useState<WelcomeScreen>("hello");
  const [{
    effectsState,
    isArch,
    isLive,
    ootbPending,
    resolutions
  }, setData] = useState<DataState>({
    isLive: false,
    isArch: false,
    ootbPending: false,
    resolutions: [],
    effectsState: false,
  });

  const loadSystemInfo = useCallback(async () => {
    const [live, arch, ootb, res, effects] = await Promise.all([
      isLiveSystem(),
      isArchSystem(),
      isOotbPending(),
      getResolutions(),
      getEffectsState(),
    ]);
    setData((prev) => ({
      ...prev,
      isLive: live,
      isArch: arch,
      ootbPending: ootb,
      resolutions: res,
      effectsState: effects
    }));
  }, []);

  const proceedFromHello = useCallback(async () => {
    if (ootbPending && !isLive) {
      await launchOotb();
      await closeWindow();
      return;
    }
    setScreen("features");
  }, [ootbPending, isLive]);

  const goNext = useCallback(() => {
    setScreen((prev) => {
      const idx = BASE_FLOW.indexOf(prev);
      if (idx !== -1 && idx < BASE_FLOW.length - 1) {
        return BASE_FLOW[idx + 1];
      }
      if (prev === "sayri") return isLive ? "recovery" : "done";
      return prev;
    });
  }, [isLive]);

  const goBack = useCallback(() => {
    setScreen((prev) => {
      const idx = BASE_FLOW.indexOf(prev);
      if (idx > 0) return BASE_FLOW[idx - 1];
      return prev;
    });
  }, []);

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
