import { useState } from "react";
import { Screen } from "@/modules/ui";
import { launchDisplaySettings, setResolution } from "@/modules/core/api";
import type { Resolution } from "@/modules/core/types";
import { cn } from "@/modules/ui/utils";

interface ResolutionPageProps {
  resolutions: Resolution[];
  onContinue: () => void;
  onBack: () => void;
}

export function ResolutionPage({
  resolutions,
  onContinue,
  onBack,
}: ResolutionPageProps) {
  const [active, setActive] = useState(() => resolutions.find((r) => r.active));

  const handleSelect = async (r: Resolution) => {
    if (r.active) return;
    try {
      await setResolution(r.width, r.height);
      setActive(r);
    } catch (e) {
      console.error("Failed to set resolution:", e);
    }
  };

  return (
    <Screen
      title="Select Screen Resolution"
      subtitle="Adjust the desktop screen resolution to best fit your monitor. In virtual machine environments, opening system display settings will allow you to configure the ideal size."
      footer={
        <div className="flex w-full items-center justify-between">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </div>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="glass-grouped max-h-60 overflow-y-auto p-1">
          {resolutions.map((r) => {
            const isActive = active
              ? r.width === active.width && r.height === active.height
              : r.active;
            return (
              <button
                key={`${r.width}x${r.height}`}
                onClick={() => handleSelect(r)}
                className={cn(
                  "cursor-pointer rounded-lg px-4 py-2.5 text-[13px] transition-colors w-full text-left",
                  isActive
                    ? "bg-accent text-white"
                    : "text-text-primary hover:bg-black/5",
                )}
              >
                {r.width} x {r.height}
                {isActive && (
                  <span className="ml-2 text-[11px] opacity-70">(active)</span>
                )}
              </button>
            );
          })}
          {resolutions.length === 0 && (
            <div className="px-4 py-3 text-[13px] text-text-secondary">
              No resolutions detected via xrandr.
            </div>
          )}
        </div>
        <button
          className="btn-secondary self-start"
          onClick={() => launchDisplaySettings()}
        >
          Open Display Settings
        </button>
      </div>
    </Screen>
  );
}
