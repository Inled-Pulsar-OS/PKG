import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";

export function CompletePage() {
  const { doReboot } = useRecoveryContext();

  return (
    <Screen
      title="Restoration Complete"
      subtitle={"Pulsar OS has been successfully restored.\nYour personal files, settings, and apps in /home are intact.\n\nClick Restart to boot into your restored system."}
    >
      <div className="mb-3 text-[56px]">✅</div>
      <button
        className="rounded-lg bg-accent px-6 py-2.5 text-[14px] font-semibold text-white transition-colors hover:bg-accent-hover"
        onClick={doReboot}
      >
        Restart System
      </button>
    </Screen>
  );
}
