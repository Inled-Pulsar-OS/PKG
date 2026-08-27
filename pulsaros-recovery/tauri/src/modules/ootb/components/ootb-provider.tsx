import { createContext, useContext, type ReactNode } from "react";
import { useOotb } from "../hooks/use-ootb";

type OotbContextType = ReturnType<typeof useOotb>;

const OotbContext = createContext<OotbContextType | null>(null);

export function OotbProvider({ children }: { children: ReactNode }) {
  const value = useOotb();
  return <OotbContext.Provider value={value}>{children}</OotbContext.Provider>;
}

export function useOotbContext() {
  const ctx = useContext(OotbContext);
  if (!ctx)
    throw new Error("useOotbContext must be used within OotbProvider");
  return ctx;
}
