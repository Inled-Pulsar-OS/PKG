import { Screen, Icon } from "@/modules/ui";

interface InstallerErrorPageProps {
  error: string | null;
  onBack: () => void;
  onRetry: () => void;
}

export function InstallerErrorPage({
  error,
  onBack,
  onRetry,
}: InstallerErrorPageProps) {
  return (
    <Screen
      title="Installation Failed"
      subtitle="An error occurred during installation."
      footer={
        <div className="flex gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onRetry}>
            Try Again
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
