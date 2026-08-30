import { createContext, useContext, type ReactNode } from "react";
import { useWelcome } from "@/modules/core/hooks/use-welcome";

type ContextType = ReturnType<typeof useWelcome>;

const Context = createContext<ContextType | null>(null);

export function Provider({ children }: { children: ReactNode }) {
  const value = useWelcome();
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useWelcomeContext() {
  const ctx = useContext(Context);
  if (!ctx) throw new Error("useContext must be used within WelcomeProvider");
  return ctx;
}
