import { LifeBuoy, ArrowRight } from "lucide-react";
import { launchRecovery, closeWindow, writeSentinel } from "@/modules/core/api";

interface RecoveryPageProps {
  onContinue?: () => void;
  onBack: () => void;
}

export function RecoveryPage({ onBack }: RecoveryPageProps) {
  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-10 py-6">
          <div className="flex h-28 w-28 items-center justify-center rounded-3xl bg-apple-blue/10">
            <LifeBuoy className="h-16 w-16 text-apple-blue" strokeWidth={1.5} />
          </div>
          <h1 className="mt-6 text-center text-[32px] font-semibold leading-tight text-text-primary">
            Recovery System
          </h1>
          <p className="mt-3 max-w-120 text-center text-[15px] leading-relaxed text-text-secondary">
            You are running the Pulsar OS live environment. Repair, reset or
            roll back the system, restore a backup, or install Pulsar OS from
            the built-in recovery environment. No USB stick required.
          </p>
          <button
            className="btn-primary mt-8 flex items-center gap-2 px-6 py-3 text-[16px]"
            onClick={async () => {
              await launchRecovery();
              await closeWindow();
            }}
          >
            <LifeBuoy className="h-5 w-5" strokeWidth={2} />
            Open Pulsar Recovery
          </button>
          <p className="mt-4 max-w-90 text-center text-[12px] text-text-tertiary">
            This will open the Pulsar OS installer and system recovery
            utilities.
          </p>
        </main>

        <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8 sm:py-4">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button
            className="btn-primary flex items-center gap-2"
            onClick={async () => {
              await writeSentinel();
              await closeWindow();
            }}
          >
            Finish
            <ArrowRight className="h-4 w-4" strokeWidth={2} />
          </button>
        </footer>
      </div>
    </div>
  );
}