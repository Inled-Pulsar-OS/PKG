import { cn } from "@/modules/ui/utils";
import { LockKeyhole } from "lucide-react";
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
  return (
    <button
      key={network.ssid}
      onClick={() => handleSelect(network.ssid)}
      className={cn(
        "cursor-pointer flex w-65 items-center gap-3 px-4 py-3 text-left transition-colors",
        networkSelected === network.ssid
          ? "bg-accent/10"
          : "hover:bg-accent/10",
      )}
    >
      <SignalBars signal={network.signal} />
      <span className="flex-1 truncate text-[16px] font-normal">
        {network.ssid}
      </span>
      {isSecured(network.security) && <LockKeyhole size={16} />}
    </button>
  );
}
