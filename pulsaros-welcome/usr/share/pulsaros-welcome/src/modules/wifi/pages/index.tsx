import { Layout } from "@/modules/ui";
import { launchWifiSettings } from "@/modules/core/api";

interface WifiPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function WifiPage({ onContinue, onBack }: WifiPageProps) {
  return (
    <Layout
      title="Connect to Wi-Fi"
      subtitle="Connect to your Wi-Fi network to access online services, software updates, and app downloads. Pulsar OS uses NetworkManager, which manages wireless, Ethernet and mobile broadband connections automatically."
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
        onClick={() => launchWifiSettings()}
      >
        Open Wi-Fi Settings
      </button>
    </Layout>
  );
}
