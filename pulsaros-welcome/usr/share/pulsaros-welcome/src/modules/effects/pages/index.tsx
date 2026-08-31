import { Layout } from "@/modules/ui";
import { cn } from "@/modules/ui/utils";
import { DESKTOP_EFFECTS } from "../constants";

interface EffectsPageProps {
  effectsState: boolean;
  onSetEffects: (useLiquidGlass: boolean) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function EffectsPage({
  effectsState,
  onSetEffects,
  onContinue,
  onBack,
}: EffectsPageProps) {
  return (
    <Layout
      title="Desktop Special Effects"
      subtitle='Choose the desktop effect you like best. The basic "blur-my-shell" effect is perfect for older computers or those with mid-range hardware. The "Liquid Glass" effect consumes more resources but gives a premium Apple-like look.'
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
      <div className="flex flex-col gap-3">
        {DESKTOP_EFFECTS.map((effect) => (
          <button
            key={effect.id}
            className={cn(
              "glass-grouped rounded-xl p-4 text-left transition-all cursor-pointer",
              effect.id === "blur-shell" &&
                !effectsState &&
                "ring-2 ring-accent",
              effect.id === "liquid-glass" &&
                effectsState &&
                "ring-2 ring-accent",
            )}
            onClick={() => onSetEffects(effect.value)}
          >
            <div className="text-[14px] font-semibold text-text-primary">
              {effect.name}
            </div>
            <div className="mt-1 text-[12px] text-text-secondary">
              {effect.description}
            </div>
          </button>
        ))}

        <p className="text-[12px] text-red-500">
          <strong>Warning:</strong> Liquid Glass consumes significant system
          resources. May cause lag on older GPUs or virtual machines.
        </p>
      </div>
    </Layout>
  );
}
