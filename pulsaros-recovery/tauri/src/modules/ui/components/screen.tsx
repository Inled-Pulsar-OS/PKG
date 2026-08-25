import type { ReactNode } from "react";

interface ScreenProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
  footer?: ReactNode;
}

export function Screen({ title, subtitle, children, footer }: ScreenProps) {
  return (
    <div className="bg-atmosphere flex h-screen w-screen items-center justify-center p-5 sm:p-8">
      <div
        data-sys-element-name="screen"
        className="glass screen-enter flex h-[75vh] max-h-[700px] min-h-[400px] w-[60vw] max-w-[800px] min-w-[500px] flex-col overflow-hidden"
      >
        {/* Header */}
        <header className="flex flex-shrink-0 flex-col items-center px-8 pt-12 pb-4 sm:pt-16">
          <h1 className="text-center text-[26px] font-semibold leading-tight text-text-primary sm:text-[28px]">
            {title}
          </h1>
          {subtitle && (
            <p className="mt-2.5 max-w-[440px] text-center text-[14px] leading-relaxed text-text-secondary sm:text-[15px]">
              {subtitle}
            </p>
          )}
        </header>

        {/* Content */}
        <main className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 pb-6">
          <div className="m-auto flex w-full max-w-[540px] flex-col">
            {children}
          </div>
        </main>

        {/* Footer */}
        {footer && (
          <footer className="flex flex-shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8 sm:py-4">
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}
