import { Icon } from "@/modules/ui";

interface ProgressBarProps {
  progress: number;
  statusText: string;
}

export function ProgressBar({ progress, statusText }: ProgressBarProps) {
  return (
    <div className="flex w-full flex-col items-center gap-4 py-12">
      <Icon name="refresh" className="h-12 w-12 text-accent" />
      <h2 className="text-[22px] font-semibold text-text-primary">
        Restoring Pulsar OS...
      </h2>
      <p className="text-[14px] text-text-secondary">{statusText}</p>
      <div className="h-2 w-full max-w-[380px] overflow-hidden rounded-full bg-black/[0.06]">
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-300"
          style={{ width: `${progress * 100}%` }}
        />
      </div>
    </div>
  );
}
