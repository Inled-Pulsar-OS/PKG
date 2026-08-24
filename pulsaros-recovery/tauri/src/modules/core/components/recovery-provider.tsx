import { createContext, useContext, type ReactNode } from "react";
import { useRecovery } from "@/modules/core/hooks/use-recovery";

type RecoveryContextType = ReturnType<typeof useRecovery>;

const RecoveryContext = createContext<RecoveryContextType | null>(null);

export function RecoveryProvider({ children }: { children: ReactNode }) {
  const value = useRecovery();
  return (
    <RecoveryContext.Provider value={value}>{children}</RecoveryContext.Provider>
  );
}

export function useRecoveryContext() {
  const ctx = useContext(RecoveryContext);
  if (!ctx) throw new Error("useRecoveryContext must be used within RecoveryProvider");
  return ctx;
}
