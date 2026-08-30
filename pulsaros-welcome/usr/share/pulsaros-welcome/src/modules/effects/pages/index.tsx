import { Screen } from "@/modules/ui";

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
    <Screen
      title="Desktop Special Effects"
      subtitle='Choose the desktop effect you like best. The basic "blur-my-shell" effect is perfect for older computers or those with mid-range hardware. The "Liquid Glass" effect consumes more resources but gives a premium Apple-like look.'
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
        <button
          className={`glass-grouped rounded-xl p-4 text-left transition-all ${
            !effectsState ? "ring-2 ring-accent" : ""
          }`}
          onClick={() => onSetEffects(false)}
        >
          <div className="text-[14px] font-semibold text-text-primary">
            Enable Blur my Shell
          </div>
          <div className="mt-1 text-[12px] text-text-secondary">
            Standard performance — Recommended
          </div>
        </button>

        <button
          className={`glass-grouped rounded-xl p-4 text-left transition-all ${
            effectsState ? "ring-2 ring-accent" : ""
          }`}
          onClick={() => onSetEffects(true)}
        >
          <div className="text-[14px] font-semibold text-text-primary">
            Enable Liquid Glass
          </div>
          <div className="mt-1 text-[12px] text-text-secondary">
            Premium Apple look — High Resource Consumption!
          </div>
        </button>

        <p className="text-[12px] text-red-500">
          <strong>Warning:</strong> Liquid Glass consumes significant system
          resources. May cause lag on older GPUs or virtual machines.
        </p>
      </div>
    </Screen>
  );
}
