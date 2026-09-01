import { launchApp, openUrl } from "@/modules/core/api";
import { CROSS_PLATFORM_APPS, KDE_CONNECT_QR, KDE_CONNECT_URL } from "../constants";

interface CompatibilityPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function CompatibilityPage({
  onContinue,
  onBack,
}: CompatibilityPageProps) {
  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <header className="flex shrink-0 flex-col items-center px-8 pt-12 pb-2 select-none">
          <h1 className="text-center text-[26px] font-semibold leading-tight text-text-primary sm:text-[30px]">
            Connected everywhere
          </h1>
          <p className="mt-2.5 max-w-130 text-center text-[14px] leading-relaxed text-text-secondary sm:text-[15px]">
            Your other systems aren't islands. Pulsar OS bridges macOS, Windows,
            and Android, natively.
          </p>
        </header>

        <main className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 pb-4">
          <div className="m-auto flex w-full max-w-160 flex-col gap-4">
            {/* Platform apps */}
            <div className="grid grid-cols-2 gap-4">
              {CROSS_PLATFORM_APPS.map((app) => (
                <div
                  key={app.id}
                  className="glass-grouped flex flex-col items-center rounded-2xl p-5 text-center"
                >
                  <img
                    src={app.logo}
                    alt={app.name}
                    className="mb-3 h-14 w-14 rounded-xl bg-white/60 object-contain p-1"
                  />
                  <div className="text-[15px] font-semibold text-text-primary">
                    {app.name}
                  </div>
                  <p className="mt-1.5 text-[12px] leading-snug text-text-secondary">
                    {app.description}
                  </p>
                  <button
                    className="btn-primary mt-4"
                    onClick={() => launchApp(app.launch, app.fallback)}
                  >
                    Open in Pulsar OS
                  </button>
                </div>
              ))}
            </div>

            {/* KDE Connect QR */}
            <div className="glass-grouped flex items-center gap-5 rounded-2xl p-5">
              <img
                src={KDE_CONNECT_QR}
                alt="KDE Connect download QR"
                className="h-28 w-28 rounded-lg bg-white p-2"
              />
              <div className="flex flex-col items-start">
                <div className="text-[15px] font-semibold text-text-primary">
                  KDE Connect for your phone
                </div>
                <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                  Scan the code to install KDE Connect and pair your phone with
                  Pulsar OS. Notifications, file sharing and remote control.
                </p>
                <button
                  className="btn-secondary mt-3"
                  onClick={() => openUrl(KDE_CONNECT_URL)}
                >
                  View download page
                </button>
              </div>
            </div>
          </div>
        </main>

        <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8 sm:py-4">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </footer>
      </div>
    </div>
  );
}