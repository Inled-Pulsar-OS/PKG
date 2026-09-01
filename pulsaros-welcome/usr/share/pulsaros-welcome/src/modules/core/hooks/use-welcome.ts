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

  /**
   * After the hello animation the user clicks Continue:
   * - If the OOTB setup assistant is pending (first boot after install),
   *   launch pulsaros-ootb and let it take over.
   * - Otherwise proceed through the normal slideshow.
   */
  const proceedFromHello = useCallback(async () => {
    if (ootbPending) {
      await launchOotb();
      await closeWindow();
      return;
    }
    setScreen("features");
  }, [ootbPending]);

  const goNext = useCallback(() => {
    setScreen((prev) => {
      switch (prev) {
        case "hello":
          return "features";
        case "features":
          return "compatibility";
        case "compatibility":
          return "settings";
        case "settings":
          return "sayri";
        case "sayri":
          return isLive ? "recovery" : "done";
        case "recovery":
          return "done";
        default:
          return "done";
      }
    });
  }, [isLive]);

  const goBack = useCallback(() => {
    setScreen((prev) => {
      switch (prev) {
        case "features":
          return "hello";
        case "compatibility":
          return "features";
        case "settings":
          return "compatibility";
        case "sayri":
          return "settings";
        case "recovery":
          return "sayri";
        default:
          return "hello";
      }
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