import { Screen, Icon } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";

export function ErrorPage() {
  const { error, goBackFromError } = useRecoveryContext();

  return (
    <Screen
      title="Restoration Failed"
      subtitle="An error occurred during system restoration."
      footer={
        <button className="btn-secondary" onClick={goBackFromError}>
          Back to Utilities
        </button>
      }
    >
      <div className="flex flex-col items-center gap-4">
        <Icon name="x" className="h-14 w-14 text-red-500" />
        <div className="max-h-[140px] w-full overflow-y-auto rounded-lg border border-border bg-black/[0.04] p-3 font-mono text-[12px] text-red-500 break-all">
          {error ?? "Unknown error"}
        </div>
      </div>
    </Screen>
  );
}
