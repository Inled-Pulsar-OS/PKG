import type { ReactNode } from "react";

interface LayoutProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
  footer?: ReactNode;
  fullscreen?: boolean;
}

export function Layout({
  title,
  subtitle,
  children,
  footer,
  fullscreen = false,
}: LayoutProps) {
  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div
        data-sys-element-name="screen"
        className={`screen-enter glass flex w-full max-w-[860px] flex-col overflow-hidden ${
          fullscreen ? "h-full" : "h-[86vh] max-h-[740px]"
        }`}
      >
        {/* Header */}
        <header className="flex shrink-0 flex-col items-center px-8 pt-12 pb-4 sm:pt-14 select-none">
          <h1 className="text-center text-[26px] font-semibold leading-tight text-text-primary sm:text-[30px]">
            {title}
          </h1>
          {subtitle && (
            <p className="mt-2.5 max-w-130 text-center text-[14px] leading-relaxed text-text-secondary sm:text-[15px]">
              {subtitle}
            </p>
          )}
        </header>

        {/* Content */}
        <main className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 pb-6">
          <div className="m-auto flex w-full max-w-150 flex-col">
            {children}
          </div>
        </main>

        {/* Footer */}
        {footer && (
          <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8 sm:py-4">
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}