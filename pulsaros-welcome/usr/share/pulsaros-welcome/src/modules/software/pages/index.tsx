import { Screen } from "@/modules/ui";
import { launchApp } from "@/modules/core/api";

interface SoftwarePageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function SoftwarePage({
  onContinue,
  onBack,
}: SoftwarePageProps) {
  return (
    <Screen
      title="Installing Applications"
      subtitle="Pulsar OS Software Center allows you to easily install Linux applications from both the official repositories and Flathub. Additionally, you can download standard .deb installation files from your web browser and install them by simply double-clicking on them."
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
      <button
        className="btn-secondary self-start"
        onClick={() => launchApp("appinstall")}
      >
        Open Software Center
      </button>
    </Screen>
  );
}
