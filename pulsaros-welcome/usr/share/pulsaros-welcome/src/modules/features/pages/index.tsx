import { Layout } from "@/modules/ui";
import { FEATURES } from "../constants";

interface FeaturesPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function FeaturesPage({ onContinue, onBack }: FeaturesPageProps) {
  return (
    <Layout
      title="Pulsar OS Features"
      subtitle="Everything included in your new operating system. No extra setup needed."
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
      <div className="grid grid-cols-2 gap-3">
        {FEATURES.map((feature) => (
          <div
            key={feature.name}
            className="glass-grouped rounded-xl p-4 transition-shadow"
          >
            <div className="text-[20px]">{feature.icon}</div>
            <div className="mt-2 text-[13px] font-semibold text-text-primary">
              {feature.name}
            </div>
            <div className="mt-1 text-[11px] leading-snug text-text-secondary">
              {feature.description}
            </div>
          </div>
        ))}
      </div>
    </Layout>
  );
}
