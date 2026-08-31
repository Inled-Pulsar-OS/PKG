import { Layout } from "@/modules/ui";
import { launchApp } from "@/modules/core/api";

interface GpuPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function GpuPage({ onContinue, onBack }: GpuPageProps) {
  return (
    <Layout
      title="GPU Driver Manager"
      subtitle="Pulsar OS detects your GPU automatically and recommends the best open-source or proprietary driver for it. Use Driver Manager to install, switch, or remove GPU drivers. Package conflicts can be resolved directly from the app."
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
      <button
        className="btn-secondary self-start"
        onClick={() => launchApp("driverman-gui", "driverman")}
      >
        Open Driver Manager
      </button>
    </Layout>
  );
}
