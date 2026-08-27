import { Screen, Icon } from "@/modules/ui";

interface InstallerWelcomePageProps {
  onContinue: () => void;
}

export function InstallerWelcomePage({ onContinue }: InstallerWelcomePageProps) {
  return (
    <Screen
      title="Install Pulsar OS"
      subtitle="This will install Pulsar OS on your computer. All data on the selected disk will be erased."
      footer={
        <div className="flex w-full items-center justify-end">
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </div>
      }
    >
      <div className="flex flex-col items-center gap-6 py-8">
        <div className="flex h-20 w-20 items-center justify-center rounded-2xl bg-accent/10">
          <Icon name="disk" className="h-10 w-10 text-accent" />
        </div>
        <div className="text-center">
          <p className="text-[14px] text-text-secondary">
            The installer will partition your disk, create Btrfs subvolumes,
            and copy the system files.
          </p>
        </div>
      </div>
    </Screen>
  );
}
