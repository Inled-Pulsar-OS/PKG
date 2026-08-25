import { Screen, Icon } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";

export function CompletePage() {
  const { doReboot } = useRecoveryContext();

  return (
    <Screen
      title="Restoration Complete"
      subtitle="Pulsar OS has been successfully restored. Your personal files, settings, and apps in /home are intact."
      footer={
        <>
          <div />
          <button className="btn-primary" onClick={doReboot}>
            Restart System
          </button>
        </>
      }
    >
      <div className="flex justify-center py-6">
        <Icon name="check" className="h-14 w-14 text-accent" />
      </div>
    </Screen>
  );
}
