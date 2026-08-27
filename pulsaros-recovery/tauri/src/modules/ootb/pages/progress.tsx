import { useEffect, useRef } from "react";
import { Screen, Icon } from "@/modules/ui";

interface OotbProgressPageProps {
  progress: number;
  statusText: string;
  logs: string[];
}

export function OotbProgressPage({
  progress,
  statusText,
  logs,
}: OotbProgressPageProps) {
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <Screen title="">
      <div className="flex w-full flex-col items-center gap-4 py-12">
        <Icon name="refresh" className="h-12 w-12 text-accent" />
        <h2 className="text-[22px] font-semibold text-text-primary">
          Setting Up Pulsar OS...
        </h2>
        <p className="text-[14px] text-text-secondary">{statusText}</p>
        <div className="h-2 w-full max-w-[380px] overflow-hidden rounded-full bg-black/[0.06]">
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-300"
            style={{ width: `${progress * 100}%` }}
          />
        </div>
      </div>
      {logs.length > 0 && (
        <div className="mt-4 h-[180px] w-full max-w-[480px] overflow-y-auto rounded-lg border border-border bg-black/4 p-3">
          <pre className="whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed text-text-secondary">
            {logs.map((line, i) => (
              <div key={i}>{line}</div>
            ))}
            <div ref={logEndRef} />
          </pre>
        </div>
      )}
    </Screen>
  );
}
