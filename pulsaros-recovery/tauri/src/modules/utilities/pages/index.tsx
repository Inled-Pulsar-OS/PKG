import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { UTILITIES } from "../data/utilities";
import { UtilityRow } from "../components/utility-row";

export function UtilitiesPage() {
  const { selectedAction, selectAction, continueFromUtilities } = useRecoveryContext();

  return (
    <Screen
      title="Pulsar OS Recovery Utilities"
      subtitle="Select a recovery utility to restore or repair your system."
    >
      <div className="w-full overflow-hidden rounded-xl border border-border">
        {UTILITIES.map((u) => (
          <UtilityRow
            key={u.id}
            option={u}
            selected={selectedAction === u.id}
            onSelect={selectAction}
          />
        ))}
      </div>
      <div className="mt-auto flex w-full justify-end pt-4">
        <button
          className="rounded-lg bg-accent px-6 py-2.5 text-[14px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:bg-[#38383a] disabled:text-[#636366] disabled:cursor-default"
          disabled={!selectedAction}
          onClick={continueFromUtilities}
        >
          Continue
        </button>
      </div>
    </Screen>
  );
}
