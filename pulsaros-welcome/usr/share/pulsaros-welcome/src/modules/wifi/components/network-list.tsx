import { cn } from "@/modules/ui/utils";
import type { WifiNetwork } from "../types";
import { isSecured } from "../utils";
import { LockIcon } from "lucide-react";
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
  return (
    <button
      key={network.ssid}
      onClick={() => handleSelect(network.ssid)}
      className={cn(
        "flex w-full items-center gap-3 px-4 py-3 text-left transition-colors",
        networkSelected === network.ssid ? "bg-accent/10" : "hover:bg-white/5",
      )}
    >
      <SignalBars signal={network.signal} />
      <span className="flex-1 truncate text-sm font-medium text-text-primary">
        {network.ssid}
      </span>
      {isSecured(network.security) && <LockIcon />}
    </button>
  );
}
