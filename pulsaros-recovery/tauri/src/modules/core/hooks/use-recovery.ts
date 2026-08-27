import { useState, useCallback, useRef, useEffect } from "react";
import type { BtrfsTarget, RecoveryMode, Screen } from "@/modules/core/types";
import {
  getBtrfsTargets,
  startRestore,
  launchApp,
  reboot,
  onRestoreProgress,
  onRestoreLog,
} from "@/modules/core/api";
import type { UtilityAction } from "@/modules/utilities/data/utilities";

const INTERNET_RECOVERY_URL =
  "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs";

export function useRecovery() {
  const [screen, setScreen] = useState<Screen>("utilities");
  const [selectedAction, setSelectedAction] = useState<UtilityAction | null>(
    null
  );
  const [targets, setTargets] = useState<BtrfsTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<BtrfsTarget | null>(
    null
  );
  const [recoveryMode, setRecoveryMode] = useState<RecoveryMode>("local");
  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const unlistenersRef = useRef<Array<() => void>>([]);

  const cleanupListeners = useCallback(() => {
    unlistenersRef.current.forEach((unlisten) => unlisten());
    unlistenersRef.current = [];
  }, []);

  const attachRestoreListeners = useCallback(() => {
    cleanupListeners();
    setLogs([]);
    onRestoreProgress((pct: number, status: string) => {
      setProgress(pct);
      setStatusText(status);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
    onRestoreLog((msg: string) => {
      setLogs((prev) => [...prev, msg]);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
  }, [cleanupListeners]);

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
    if (selectedAction === "terminal") {
      await launchApp("terminal");
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
    attachRestoreListeners();

    try {
      const url =
        recoveryMode === "internet" ? INTERNET_RECOVERY_URL : undefined;
      await startRestore(selectedTarget, url);
      cleanupListeners();
      setProgress(1);
      setStatusText("Complete!");
      setScreen("complete");
    } catch (e) {
      cleanupListeners();
      setError(String(e));
      setScreen("error");
    }
  }, [selectedTarget, recoveryMode, attachRestoreListeners, cleanupListeners]);

  const tryInternetRecovery = useCallback(async () => {
    if (!selectedTarget) return;
    setScreen("progress");
    setProgress(0);
    setStatusText("Downloading recovery image...");
    attachRestoreListeners();
    setRecoveryMode("internet");

    try {
      await startRestore(selectedTarget, INTERNET_RECOVERY_URL);
      cleanupListeners();
      setProgress(1);
      setStatusText("Complete!");
      setScreen("complete");
    } catch (e) {
      cleanupListeners();
      setError(String(e));
      setScreen("error");
    }
  }, [selectedTarget, attachRestoreListeners, cleanupListeners]);

  const goBack = useCallback(() => {
    cleanupListeners();
    setScreen("utilities");
    setSelectedAction(null);
    setSelectedTarget(null);
    setError(null);
  }, [cleanupListeners]);

  const goBackFromError = useCallback(() => {
    cleanupListeners();
    setScreen("utilities");
    setError(null);
  }, [cleanupListeners]);

  const doReboot = useCallback(async () => {
    await reboot();
  }, []);

  useEffect(() => {
    return () => {
      unlistenersRef.current.forEach((unlisten) => unlisten());
      unlistenersRef.current = [];
    };
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
    logs,
    selectAction,
    continueFromUtilities,
    startLocalRestoreFlow,
    launchCalamares,
    setSelectedTarget,
    startRestoreFlow,
    tryInternetRecovery,
    goBack,
    goBackFromError,
    doReboot,
  };
}
