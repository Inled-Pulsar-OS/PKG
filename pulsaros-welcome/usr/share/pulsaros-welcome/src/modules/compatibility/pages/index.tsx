import { useState } from "react";
import { launchApp, openUrl } from "@/modules/core/api";
import { CROSS_PLATFORM_APPS, FDROID_REPO } from "../constants";

interface CompatibilityPageProps {
  onContinue: () => void;
  onBack: () => void;
}

/** Steps shown above the embedded F-Droid repository. */
const ANDROID_STEPS: { text: string; code?: string }[] = [
  { text: "On your phone, install an F-Droid client: Droidify, F-Droid or IzzyOnDroid." },
  { text: "In the client, open Repositories and add the official Inled repository:", code: `${FDROID_REPO}/repo` },
  { text: "Search for Pulsar Phone and Sayri Mobile, then install them." },
  { text: "Back on your PC use “Configure Pulsar Phone” to pair both devices." },
];

export function CompatibilityPage({
  onContinue,
  onBack,
}: CompatibilityPageProps) {
  const [androidOpen, setAndroidOpen] = useState(false);

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <header className="flex shrink-0 flex-col items-center px-8 pt-12 pb-5 select-none">
          <h1 className="text-center text-[26px] font-semibold leading-tight text-text-primary sm:text-[30px]">
            Ecosystem & Native Integration
          </h1>
          <p className="mt-2.5 max-w-130 text-center text-[14px] leading-relaxed text-text-secondary sm:text-[15px]">
            Pulsar OS unifies your phone, your Windows apps, and intelligent AI directly into one seamless desktop experience.
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
                  {app.url && (
                    <button
                      className="btn-secondary mt-2"
                      onClick={() => setAndroidOpen(true)}
                    >
                      {app.urlLabel ?? "Learn more"}
                    </button>
                  )}
                </div>
              ))}
            </div>

            {/* Pulsar Phone Feature Banner */}
            <div className="glass-grouped flex items-center gap-5 rounded-2xl p-5">
              <img
                src="./logos/pulsar-phone.svg"
                alt="Pulsar Phone"
                className="h-20 w-20 rounded-xl bg-blue-500/10 p-2 object-contain"
              />
              <div className="flex flex-col items-start">
                <div className="text-[15px] font-semibold text-text-primary">
                  Connect your phone with Pulsar Phone
                </div>
                <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                  Stream your screen with zero audio lag, launch mobile apps directly on your PC, share your clipboard, and prevent sleep for 24/7 remote access.
                </p>
                <div className="mt-3 flex gap-2">
                  <button
                    className="btn-secondary"
                    onClick={() => launchApp("pulsar-vpn")}
                  >
                    Configure Pulsar Phone
                  </button>
                  <button
                    className="btn-secondary"
                    onClick={() => setAndroidOpen(true)}
                  >
                    Get the Android apps
                  </button>
                </div>
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

      {/* Android apps: embedded F-Droid repository + install instructions */}
      {androidOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          role="dialog"
          aria-modal="true"
          aria-label="Get the Android apps"
        >
          <div className="glass flex h-[88vh] w-full max-w-[900px] flex-col overflow-hidden rounded-2xl">
            <header className="flex shrink-0 items-start justify-between gap-4 px-6 pt-5 pb-3">
              <div>
                <h2 className="text-[20px] font-semibold text-text-primary">
                  Get the Android apps
                </h2>
                <p className="mt-1 text-[13px] text-text-secondary">
                  Official Inled F-Droid repository — install F-Droid or Droidify on your
                  phone and add <span className="font-mono">{FDROID_REPO}/repo</span>
                </p>
              </div>
              <button
                className="btn-secondary shrink-0"
                onClick={() => setAndroidOpen(false)}
              >
                Close
              </button>
            </header>

            <ol className="shrink-0 list-decimal space-y-1.5 px-6 pb-3 pl-10 text-[13px] leading-snug text-text-secondary">
              {ANDROID_STEPS.map((step, i) => (
                <li key={i}>
                  {step.text}
                  {step.code && (
                    <span className="ml-1.5 rounded bg-white/10 px-1.5 py-0.5 font-mono text-[12px] text-text-primary">
                      {step.code}
                    </span>
                  )}
                </li>
              ))}
            </ol>

            <iframe
              src={FDROID_REPO}
              title="Inled F-Droid repository"
              className="mx-6 mb-3 min-h-0 flex-1 rounded-xl border border-separator bg-white"
            />

            <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-separator px-6 py-3">
              <button className="btn-secondary" onClick={() => openUrl(FDROID_REPO)}>
                Open in browser
              </button>
              <button className="btn-primary" onClick={() => setAndroidOpen(false)}>
                Done
              </button>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
}
