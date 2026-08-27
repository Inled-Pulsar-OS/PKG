import { Screen, Icon } from "@/modules/ui";

interface FinishedPageProps {
  onReboot: () => void;
}

export function FinishedPage({ onReboot }: FinishedPageProps) {
  return (
    <Screen title="">
      <div className="flex flex-col items-center gap-4 py-12">
        <div className="flex h-16 w-16 items-center justify-center rounded-full bg-green-500/10">
          <Icon name="check" className="h-8 w-8 text-green-500" />
        </div>
        <h2 className="text-[22px] font-semibold text-text-primary">
          Setup Complete
        </h2>
        <p className="text-[14px] text-text-secondary">
          Pulsar OS is ready to use. Reboot to start your new system.
        </p>
        <button className="btn-primary mt-4" onClick={onReboot}>
          Reboot Now
        </button>
      </div>
    </Screen>
  );
}
