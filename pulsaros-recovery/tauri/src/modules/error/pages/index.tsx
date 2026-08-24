import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";

export function ErrorPage() {
  const { error, goBackFromError } = useRecoveryContext();

  return (
    <Screen
      title="Restoration Failed"
      subtitle="An error occurred during system restoration."
    >
      <div className="mb-3 text-[56px]">❌</div>
      <div className="mb-4 w-full max-h-[140px] overflow-y-auto rounded-lg border border-[#333] bg-[#121212] p-3 font-mono text-[12px] text-[#ff453a] break-all">
        {error ?? "Unknown error"}
      </div>
      <button
        className="rounded-lg border border-white/[0.15] bg-[#323236] px-6 py-2.5 text-[14px] font-semibold text-white transition-colors hover:bg-[#3e3e42]"
        onClick={goBackFromError}
      >
        Back to Utilities
      </button>
    </Screen>
  );
}
