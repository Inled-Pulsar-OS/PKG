import { useState, useCallback } from "react";
import type { WelcomeScreen, DataState } from "@/modules/core/types";
import {
  isLiveSystem,
  isArchSystem,
  getResolutions,
  getEffectsState,
  setEffects as apiSetEffects,
  writeSentinel,
} from "@/modules/core/api";

const WELCOME_STEPS: WelcomeScreen[] = [
  "hello",
  "intro",
  "resolution",
  "wifi",
  "bluetooth",
  "software",
  "effects",
  "gpu",
  "feedback",
];

export function useWelcome() {
  const [screen, setScreen] = useState<WelcomeScreen>("hello");
  const [{
    effectsState,
    isArch,
    isLive,
    resolutions
  }, setData] = useState<DataState>({
    isLive: false,
    isArch: false,
    resolutions: [],
    effectsState: false,
  });

  const loadSystemInfo = useCallback(async () => {
    const [live, arch, res, effects] = await Promise.all([
      isLiveSystem(),
      isArchSystem(),
      getResolutions(),
      getEffectsState(),
    ]);
    setData((prev) => ({
      ...prev,
      isLive: live,
      isArch: arch,
      resolutions: res,
      effectsState: effects
    }));
  }, []);

  const goNext = useCallback(() => {
    setScreen((prev) => {
      const idx = WELCOME_STEPS.indexOf(prev);
      return WELCOME_STEPS[Math.min(idx + 1, WELCOME_STEPS.length - 1)];
    });
  }, []);

  const goBack = useCallback(() => {
    setScreen((prev) => {
      const idx = WELCOME_STEPS.indexOf(prev);
      return WELCOME_STEPS[Math.max(idx - 1, 0)];
    });
  }, []);

  const goTo = useCallback((s: WelcomeScreen) => setScreen(s), []);

  const setEffectsValue = useCallback(async (useLiquidGlass: boolean) => {
    await apiSetEffects(useLiquidGlass);
    setData((prev) => ({ ...prev, effectsState: useLiquidGlass }));
  }, []);

  const complete = useCallback(async () => {
    await writeSentinel();
    setScreen("done");
  }, []);

  return {
    screen,
    isLive,
    isArch,
    resolutions,
    effectsState,
    loadSystemInfo,
    goNext,
    goBack,
    goTo,
    setEffects: setEffectsValue,
    complete,
  };
}
