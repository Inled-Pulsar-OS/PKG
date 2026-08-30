import { Screen } from "@/modules/ui";

interface IntroPageProps {
  isLive: boolean;
  onContinue: () => void;
}

export function IntroPage({ isLive, onContinue }: IntroPageProps) {
  return (
    <Screen
      title="Welcome to Pulsar OS"
      subtitle={
        isLive
          ? "Pulsar OS combines speed, beauty, and design into a powerful and modern operating system. This wizard will guide you through essential configurations and allow you to install or try the system."
          : "Pulsar OS combines speed, beauty, and design into a powerful and modern operating system. This Setup Assistant will guide you through the essential configurations to customize your experience on first boot."
      }
      footer={
        <div className="flex w-full items-center justify-end">
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </div>
      }
    >
      <div className="flex flex-col items-center gap-4 py-4">
        <img src="/logo.png" alt="Pulsar OS" className="h-20 w-20" />
      </div>
    </Screen>
  );
}
