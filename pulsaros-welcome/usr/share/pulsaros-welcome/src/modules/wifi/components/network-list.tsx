import { cn } from "@/modules/ui/utils";
import { Check, LockKeyhole } from "lucide-react";
import type { WifiNetwork } from "../types";
import { isSecured } from "../utils";
import { SignalBars } from "./signal-bar";

interface WifiNetworkListProps {
  network: WifiNetwork;
  networkSelected: string | null;
  handleSelect: (ssid: string) => void;
}

export function WifiNetworkList({
  network,
  networkSelected,
  handleSelect,
}: WifiNetworkListProps) {
  const isSelected = networkSelected === network.ssid;

  return (
    <button
      key={network.ssid}
      onClick={() => handleSelect(network.ssid)}
      className={cn(
        "cursor-pointer flex w-full items-center gap-3 px-4 py-3 text-left transition-all border-b border-border/40 last:border-b-0",
        isSelected
          ? "bg-apple-blue/20 text-text-primary ring-1 ring-inset ring-apple-blue/40 font-medium"
          : "hover:bg-white/5 text-text-primary",
      )}
    >
      <SignalBars signal={network.signal} />
      <span className="flex-1 truncate text-[15px]">
        {network.ssid}
      </span>
      {network.in_use && (
        <span className="text-[11px] font-medium px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 shrink-0">
          Connected
        </span>
      )}
      {isSecured(network.security) && (
        <LockKeyhole size={15} className="text-text-secondary opacity-70 shrink-0" />
      )}
      {isSelected && (
        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-apple-blue text-white shrink-0">
          <Check size={12} strokeWidth={3} />
        </div>
      )}
    </button>
  );
}
