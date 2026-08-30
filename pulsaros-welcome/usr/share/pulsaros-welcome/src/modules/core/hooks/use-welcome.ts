import { useState, useCallback } from "react";
import type { WelcomeScreen, Resolution } from "@/modules/core/types";
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
  const [isLive, setIsLive] = useState(false);
  const [isArch, setIsArch] = useState(false);
  const [resolutions, setResolutions] = useState<Resolution[]>([]);
  const [effectsState, setEffectsState] = useState(false);

  const loadSystemInfo = useCallback(async () => {
    const [live, arch, res, effects] = await Promise.all([
      isLiveSystem(),
      isArchSystem(),
      getResolutions(),
      getEffectsState(),
    ]);
    setIsLive(live);
    setIsArch(arch);
    setResolutions(res);
    setEffectsState(effects);
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
    setEffectsState(useLiquidGlass);
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
