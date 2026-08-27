import { useEffect, useRef } from "react";
import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { ProgressBar } from "../components/progress-bar";

export function ProgressPage() {
  const { progress, statusText, logs } = useRecoveryContext();
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <Screen title="">
      <ProgressBar progress={progress} statusText={statusText} />
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
