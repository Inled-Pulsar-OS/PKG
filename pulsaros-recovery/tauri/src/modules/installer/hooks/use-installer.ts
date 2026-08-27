import { useState, useCallback, useRef, useEffect } from "react";
import type { BtrfsTarget } from "@/modules/core/types";
import {
  getBtrfsTargets,
  detectBroadcom,
  startInstall,
  onInstallProgress,
  onInstallLog,
} from "@/modules/core/api";

type InstallerScreen =
  | "welcome"
  | "disk_select"
  | "broadcom"
  | "progress"
  | "complete"
  | "error";

export function useInstaller() {
  const [screen, setScreen] = useState<InstallerScreen>("welcome");
  const [targets, setTargets] = useState<BtrfsTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<BtrfsTarget | null>(
    null
  );
  const [broadcomDetected, setBroadcomDetected] = useState(false);
  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const unlistenersRef = useRef<Array<() => void>>([]);

  const cleanupListeners = useCallback(() => {
    unlistenersRef.current.forEach((unlisten) => unlisten());
    unlistenersRef.current = [];
  }, []);

  const attachInstallListeners = useCallback(() => {
    cleanupListeners();
    setLogs([]);
    onInstallProgress((pct: number, status: string) => {
      setProgress(pct);
      setStatusText(status);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
    onInstallLog((msg: string) => {
      setLogs((prev) => [...prev, msg]);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
  }, [cleanupListeners]);

  const goToWelcome = useCallback(() => {
    setScreen("welcome");
  }, []);

  const goToDiskSelect = useCallback(async () => {
    const found = await getBtrfsTargets();
    setTargets(found);
    setSelectedTarget(null);
    setScreen("disk_select");
  }, []);

  const goToBroadcom = useCallback(async () => {
    const detected = await detectBroadcom();
    setBroadcomDetected(detected);
    setScreen("broadcom");
  }, []);

  const startInstallation = useCallback(
    async (broadcom: boolean) => {
      if (!selectedTarget) return;
      setScreen("progress");
      setProgress(0);
      setStatusText("Preparing installation...");
      attachInstallListeners();

      try {
        await startInstall(selectedTarget.disk_path, broadcom);
        cleanupListeners();
        setProgress(1);
        setStatusText("Installation complete!");
        setScreen("complete");
      } catch (e) {
        cleanupListeners();
        setError(String(e));
        setScreen("error");
      }
    },
    [selectedTarget, attachInstallListeners, cleanupListeners]
  );

  const goBack = useCallback(() => {
    cleanupListeners();
    setScreen("welcome");
    setSelectedTarget(null);
    setError(null);
  }, [cleanupListeners]);

  const goBackFromError = useCallback(() => {
    cleanupListeners();
    setScreen("disk_select");
    setError(null);
  }, [cleanupListeners]);

  useEffect(() => {
    return () => {
      unlistenersRef.current.forEach((unlisten) => unlisten());
      unlistenersRef.current = [];
    };
  }, []);

  return {
    screen,
    targets,
    selectedTarget,
    broadcomDetected,
    progress,
    statusText,
    error,
    logs,
    setSelectedTarget,
    goToWelcome,
    goToDiskSelect,
    goToBroadcom,
    startInstallation,
    goBack,
    goBackFromError,
  };
}
