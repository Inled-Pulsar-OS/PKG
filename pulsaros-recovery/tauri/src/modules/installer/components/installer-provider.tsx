import { createContext, useContext, type ReactNode } from "react";
import { useInstaller } from "../hooks/use-installer";

type InstallerContextType = ReturnType<typeof useInstaller>;

const InstallerContext = createContext<InstallerContextType | null>(null);

export function InstallerProvider({ children }: { children: ReactNode }) {
  const value = useInstaller();
  return (
    <InstallerContext.Provider value={value}>
      {children}
    </InstallerContext.Provider>
  );
}

export function useInstallerContext() {
  const ctx = useContext(InstallerContext);
  if (!ctx)
    throw new Error(
      "useInstallerContext must be used within InstallerProvider"
    );
  return ctx;
}
