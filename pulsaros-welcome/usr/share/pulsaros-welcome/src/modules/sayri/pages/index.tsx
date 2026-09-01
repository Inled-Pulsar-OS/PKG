import { Sparkles, ShieldCheck, Store } from "lucide-react";
import { openUrl } from "@/modules/core/api";

interface SayriPageProps {
  onContinue: () => void;
  onBack: () => void;
}

const STORE_URL = "https://store-os.inled.es";

export function SayriPage({ onContinue, onBack }: SayriPageProps) {
  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-10 py-6">
          <img
            src="./logos/sayri.png"
            alt="Sayri"
            className="h-28 w-28 rounded-3xl shadow-lg"
          />
          <h1 className="mt-6 text-center text-[32px] font-semibold leading-tight text-text-primary">
            Meet Sayri
          </h1>
          <p className="mt-3 max-w-130 text-center text-[15px] leading-relaxed text-text-secondary">
            Your AI assistant, built into Pulsar OS. Think of it as an agent
            like OpenClaw — but with permissions and security baked in. Sayri
            uses the model you want and acts through its own skills, plugins,
            extensions and apps, always respecting your control.
          </p>

          <div className="mt-8 grid w-full max-w-135 grid-cols-1 gap-4 sm:grid-cols-3">
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-5 text-center">
              <Sparkles className="mb-3 h-8 w-8 text-apple-blue" strokeWidth={2} />
              <div className="text-[14px] font-semibold text-text-primary">
                Skills & Agents
              </div>
              <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                Natural command agents that run tasks safely.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-5 text-center">
              <ShieldCheck className="mb-3 h-8 w-8 text-apple-blue" strokeWidth={2} />
              <div className="text-[14px] font-semibold text-text-primary">
                Permissions & Security
              </div>
              <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                Every action is sandboxed and under your control.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-5 text-center">
              <Store className="mb-3 h-8 w-8 text-apple-blue" strokeWidth={2} />
              <div className="text-[14px] font-semibold text-text-primary">
                Store
              </div>
              <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                Skills, plugins, extensions and apps.
              </p>
            </div>
          </div>

          <button
            className="btn-primary mt-8"
            onClick={() => openUrl(STORE_URL)}
          >
            Explore the Sayri Store
          </button>
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