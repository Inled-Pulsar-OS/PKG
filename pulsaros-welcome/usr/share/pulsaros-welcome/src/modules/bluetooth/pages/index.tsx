import { Screen } from "@/modules/ui";
import { launchBluetoothSettings } from "@/modules/core/api";

interface BluetoothPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function BluetoothPage({
  onContinue,
  onBack,
}: BluetoothPageProps) {
  return (
    <Screen
      title="Set Up Bluetooth Connection"
      subtitle="Connect wireless controllers, headphones, keyboards, or mice. Pulsar OS scans and listens for both Bluetooth Low Energy (BLE) and classic Bluetooth devices simultaneously for maximum hardware compatibility."
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
        onClick={() => launchBluetoothSettings()}
      >
        Configure Bluetooth Devices
      </button>
    </Screen>
  );
}
