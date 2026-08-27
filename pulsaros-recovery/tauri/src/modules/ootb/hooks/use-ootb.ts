import { useState, useCallback, useRef, useEffect } from "react";
import {
  getOotbData,
  startOotbSetup,
  ootbFinalCleanup,
  onOotbProgress,
  onOotbLog,
} from "@/modules/core/api";

export type Theme = "light" | "dark";

type OotbScreen =
  | "country"
  | "language"
  | "keymap"
  | "timezone"
  | "account"
  | "theme"
  | "progress"
  | "finished"
  | "error";

interface Account {
  fullName: string;
  username: string;
  password: string;
  avatar: string;
}

export function useOotb() {
  const [screen, setScreen] = useState<OotbScreen>("country");
  const [countries, setCountries] = useState<string[]>([]);
  const [languages, setLanguages] = useState<string[]>([]);
  const [keymaps, setKeymaps] = useState<string[]>([]);
  const [timezones, setTimezones] = useState<string[]>([]);
  const [avatars, setAvatars] = useState<string[]>([]);

  const [selectedCountry, setSelectedCountry] = useState<string | null>(null);
  const [selectedLanguage, setSelectedLanguage] = useState<string | null>(null);
  const [selectedKeymap, setSelectedKeymap] = useState<string | null>(null);
  const [selectedTimezone, setSelectedTimezone] = useState<string | null>(null);
  const [account, setAccount] = useState<Account | null>(null);

  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const unlistenersRef = useRef<Array<() => void>>([]);

  const cleanupListeners = useCallback(() => {
    unlistenersRef.current.forEach((unlisten) => unlisten());
    unlistenersRef.current = [];
  }, []);

  const attachOotbListeners = useCallback(() => {
    cleanupListeners();
    setLogs([]);
    onOotbProgress((pct: number, status: string) => {
      setProgress(pct);
      setStatusText(status);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
    onOotbLog((msg: string) => {
      setLogs((prev) => [...prev, msg]);
    }).then((unlisten: () => void) => unlistenersRef.current.push(unlisten));
  }, [cleanupListeners]);

  const loadData = useCallback(async () => {
    const data = await getOotbData();
    setCountries(data.countries);
    setLanguages(data.languages);
    setKeymaps(data.keymaps);
    setTimezones(data.timezones);
    setAvatars(data.avatars);
  }, []);

  const goToLanguage = useCallback(() => setScreen("language"), []);
  const goToKeymap = useCallback(() => setScreen("keymap"), []);
  const goToTimezone = useCallback(() => setScreen("timezone"), []);
  const goToAccount = useCallback(() => setScreen("account"), []);

  const goBackToCountry = useCallback(() => setScreen("country"), []);
  const goBackToLanguage = useCallback(() => setScreen("language"), []);
  const goBackToKeymap = useCallback(() => setScreen("keymap"), []);
  const goBackToTimezone = useCallback(() => setScreen("timezone"), []);

  const runSetup = useCallback(
    async (acct: Account) => {
      setAccount(acct);
      setScreen("progress");
      setProgress(0);
      setStatusText("Preparing setup...");
      attachOotbListeners();

      try {
        await startOotbSetup({
          fullname: acct.fullName,
          username: acct.username,
          password: acct.password,
          language: selectedLanguage!,
          keymap: selectedKeymap!,
          timezone: selectedTimezone!,
          avatarPath: acct.avatar,
        });
        cleanupListeners();
        setProgress(1);
        setStatusText("Setup complete!");
        setScreen("finished");
      } catch (e) {
        cleanupListeners();
        setError(String(e));
        setScreen("error");
      }
    },
    [
      selectedLanguage,
      selectedKeymap,
      selectedTimezone,
      attachOotbListeners,
      cleanupListeners,
    ]
  );

  const reboot = useCallback(async () => {
    await ootbFinalCleanup(account?.username ?? "");
  }, [account]);

  useEffect(() => {
    return () => {
      unlistenersRef.current.forEach((unlisten) => unlisten());
      unlistenersRef.current = [];
    };
  }, []);

  return {
    screen,
    countries,
    languages,
    keymaps,
    timezones,
    avatars,
    selectedCountry,
    selectedLanguage,
    selectedKeymap,
    selectedTimezone,
    progress,
    statusText,
    error,
    logs,
    loadData,
    setSelectedCountry,
    setSelectedLanguage,
    setSelectedKeymap,
    setSelectedTimezone,
    goToLanguage,
    goToKeymap,
    goToTimezone,
    goToAccount,
    goBackToCountry,
    goBackToLanguage,
    goBackToKeymap,
    goBackToTimezone,
    runSetup,
    reboot,
  };
}
