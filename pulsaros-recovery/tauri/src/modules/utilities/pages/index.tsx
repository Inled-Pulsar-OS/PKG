import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { UTILITIES } from "../data/utilities";
import { UtilityRow } from "../components/utility-row";

export function UtilitiesPage() {
  const { selectedAction, selectAction, continueFromUtilities } = useRecoveryContext();

  return (
    <Screen
      title="Pulsar OS Recovery"
      subtitle="Select a recovery utility to restore or repair your system."
      footer={
        <>
          <div />
          <button
            className="btn-primary"
            disabled={!selectedAction}
            onClick={continueFromUtilities}
          >
            Continue
          </button>
        </>
      }
    >
      <div className="glass-grouped overflow-hidden">
        {UTILITIES.map((u) => (
          <UtilityRow
            key={u.id}
            option={u}
            selected={selectedAction === u.id}
            onSelect={selectAction}
          />
        ))}
      </div>
    </Screen>
  );
}
