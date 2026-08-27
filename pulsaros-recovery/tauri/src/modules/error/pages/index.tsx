import { Screen, Icon } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";

export function ErrorPage() {
  const { error, goBackFromError, tryInternetRecovery } = useRecoveryContext();

  return (
    <Screen
      title="Restoration Failed"
      subtitle="An error occurred during system restoration."
      footer={
        <div className="flex gap-2">
          <button className="btn-secondary" onClick={goBackFromError}>
            Back to Utilities
          </button>
          <button className="btn-primary" onClick={tryInternetRecovery}>
            Try Internet Recovery
          </button>
        </div>
      }
    >
      <div className="flex flex-col items-center gap-4">
        <Icon name="x" className="h-14 w-14 text-red-500" />
        <div className="min-w-35 w-full overflow-y-auto rounded-lg border border-border bg-black/4 p-3 font-mono text-[12px] text-red-500 break-all">
          {error ?? "Unknown error"}
        </div>
      </div>
    </Screen>
  );
}
