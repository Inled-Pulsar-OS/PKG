import type { ReactNode } from "react";

interface ScreenProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
}

export function Screen({ title, subtitle, children }: ScreenProps) {
  return (
    <div className="flex h-screen w-screen items-center justify-center bg-bg">
      <div className="flex w-[560px] min-h-[450px] flex-col items-center rounded-[18px] border border-border-strong bg-card p-7 shadow-[0_20px_60px_rgba(0,0,0,0.8)]">
        {title && (
          <h1 className="mt-1 mb-1 text-center text-2xl font-bold text-white">
            {title}
          </h1>
        )}
        {subtitle && (
          <p className="mb-4 text-center text-[13px] leading-relaxed text-text-secondary whitespace-pre-line">
            {subtitle}
          </p>
        )}
        {children}
      </div>
    </div>
  );
}
