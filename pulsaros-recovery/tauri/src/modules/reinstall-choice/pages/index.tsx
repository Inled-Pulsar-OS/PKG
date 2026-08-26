import { useState } from "react";
import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { cn } from "@/modules/ui/utils";

type ReinstallMode = "macos" | "calamares";

const OPTIONS = [
  {
    id: "macos" as const,
    title: "MacOS like UI",
    badge: "Recommended",
    desc: "Clean reinstall using the built-in recovery tool. Your files in /home are kept.",
    icon: "/icons/reinstall.png",
  },
  {
    id: "calamares" as const,
    title: "Dual boot",
    badge: "More reliable",
    desc: "Install alongside your current system using the Calamares installer.",
    icon: "/icons/calamares.png",
  },
];

export function ReinstallChoicePage() {
  const { startLocalRestoreFlow, launchCalamares, goBack } =
    useRecoveryContext();
  const [choice, setChoice] = useState<ReinstallMode>("macos");

  return (
    <Screen
      title="Reinstall Pulsar OS"
      subtitle="Choose how you want to reinstall the system."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={goBack}>
            Back
          </button>
          <button
            className="btn-primary"
            onClick={
              choice === "macos" ? startLocalRestoreFlow : launchCalamares
            }
          >
            Continue
          </button>
        </div>
      }
    >
      <div className="flex justify-center gap-3">
        {OPTIONS.map((o) => {
          const selected = choice === o.id;
          return (
            <button
              key={o.id}
              onClick={() => setChoice(o.id)}
              className={cn(
                "flex flex-1 flex-col items-center gap-2 rounded-xl border px-5 py-5 text-center transition bg-white/60 cursor-pointer",
                {
                  "border-accent ring-2 ring-accent": selected,
                  "border-border hover:border-text-secondary": !selected,
                },
              )}
            >
              <img src={o.icon} alt="" className="h-10 w-10 object-contain" />
              <span className="text-sm font-semibold text-texty">
                {o.title}
              </span>
              <span className="text-xs text-accent">{o.badge}</span>
              <span className="text-xs leading-relaxed text-text-secondary">
                {o.desc}
              </span>
            </button>
          );
        })}
      </div>
    </Screen>
  );
}
