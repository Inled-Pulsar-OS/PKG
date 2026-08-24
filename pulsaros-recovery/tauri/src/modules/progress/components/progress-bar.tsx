interface ProgressBarProps {
  progress: number;
  statusText: string;
}

export function ProgressBar({ progress, statusText }: ProgressBarProps) {
  return (
    <div className="flex w-full flex-col items-center gap-2.5">
      <div className="text-[56px]">⚙️</div>
      <h2 className="text-2xl font-bold text-white">Restoring Pulsar OS...</h2>
      <p className="text-[13px] text-[#aeaeb2]">{statusText}</p>
      <div className="h-2 w-[380px] overflow-hidden rounded-full bg-[#3a3a3c]">
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-300"
          style={{ width: `${progress * 100}%` }}
        />
      </div>
    </div>
  );
}
