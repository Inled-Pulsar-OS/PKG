import { useState, useCallback } from "react";
import type { BtrfsTarget, RecoveryMode, Screen } from "@/modules/core/types";
import { getBtrfsTargets, startRestore, launchApp, reboot } from "@/modules/core/api";

export type UtilityAction = "backup" | "reinstall" | "internet" | "disk";

const INTERNET_RECOVERY_URL =
  "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs";

export function useRecovery() {
  const [screen, setScreen] = useState<Screen>("utilities");
  const [selectedAction, setSelectedAction] = useState<UtilityAction | null>(null);
  const [targets, setTargets] = useState<BtrfsTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<BtrfsTarget | null>(null);
  const [recoveryMode, setRecoveryMode] = useState<RecoveryMode>("local");
  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const selectAction = useCallback((action: UtilityAction) => {
    setSelectedAction(action);
  }, []);

  const continueFromUtilities = useCallback(async () => {
    if (!selectedAction) return;

    if (selectedAction === "backup") {
      await launchApp("timeshift");
      return;
    }
    if (selectedAction === "disk") {
      await launchApp("gparted");
      return;
    }
    if (selectedAction === "reinstall") {
      setScreen("reinstall_choice");
      return;
    }

    setRecoveryMode("internet");
    const found = await getBtrfsTargets();
    setTargets(found);
    setSelectedTarget(null);
    setScreen("target_select");
  }, [selectedAction]);

  const startLocalRestoreFlow = useCallback(async () => {
    setRecoveryMode("local");
    const found = await getBtrfsTargets();
    setTargets(found);
    setSelectedTarget(null);
    setScreen("target_select");
  }, []);

  const launchCalamares = useCallback(async () => {
    await launchApp("calamares");
    setScreen("utilities");
    setSelectedAction(null);
  }, []);

  const startRestoreFlow = useCallback(async () => {
    if (!selectedTarget) return;
    setScreen("progress");
    setProgress(0);
    setStatusText("Preparing...");

    try {
      const url = recoveryMode === "internet" ? INTERNET_RECOVERY_URL : undefined;
      await startRestore(selectedTarget, url);
      setProgress(1);
      setStatusText("Complete!");
      setScreen("complete");
    } catch (e) {
      setError(String(e));
      setScreen("error");
    }
  }, [selectedTarget, recoveryMode]);

  const goBack = useCallback(() => {
    setScreen("utilities");
    setSelectedAction(null);
    setSelectedTarget(null);
    setError(null);
  }, []);

  const goBackFromError = useCallback(() => {
    setScreen("utilities");
    setError(null);
  }, []);

  const doReboot = useCallback(async () => {
    await reboot();
  }, []);

  return {
    screen,
    selectedAction,
    targets,
    selectedTarget,
    recoveryMode,
    progress,
    statusText,
    error,
    selectAction,
    continueFromUtilities,
    startLocalRestoreFlow,
    launchCalamares,
    setSelectedTarget,
    startRestoreFlow,
    goBack,
    goBackFromError,
    doReboot,
  };
}
