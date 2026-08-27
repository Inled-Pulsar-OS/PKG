import { Screen } from "@/modules/ui";
import type { Theme } from "../hooks/use-ootb";

interface ThemePageProps {
  selected: Theme;
  onSelect: (theme: Theme) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function ThemePage({ selected, onSelect, onContinue, onBack }: ThemePageProps) {
  return (
    <Screen
      title="Select Theme"
      subtitle="Choose your preferred appearance."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </div>
      }
    >
      <div className="flex justify-center gap-4">
        {(["light", "dark"] as Theme[]).map((theme) => (
          <button
            key={theme}
            className={`flex h-24 w-36 flex-col items-center justify-center gap-2 rounded-xl border transition-all ${
              selected === theme
                ? "border-accent/40 ring-2 ring-accent/20 bg-white/80"
                : "border-border bg-white/60 hover:bg-white/80"
            }`}
            onClick={() => onSelect(theme)}
          >
            <div
              className={`h-10 w-14 rounded-md ${
                theme === "light"
                  ? "bg-gradient-to-b from-gray-100 to-gray-200"
                  : "bg-gradient-to-b from-gray-700 to-gray-900"
              }`}
            />
            <span className="text-[13px] font-medium capitalize text-text-primary">
              {theme}
            </span>
          </button>
        ))}
      </div>
    </Screen>
  );
}
