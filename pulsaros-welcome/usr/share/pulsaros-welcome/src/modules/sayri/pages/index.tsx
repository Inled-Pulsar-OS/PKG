import { Sparkles, ShieldCheck, ShoppingBag, ExternalLink } from "lucide-react";
import { openUrl, launchApp } from "@/modules/core/api";

interface SayriPageProps {
  onContinue: () => void;
  onBack: () => void;
}

const STORE_URL = "https://store-os.inled.es";

export function SayriPage({ onContinue, onBack }: SayriPageProps) {
  const handleOpenStore = async () => {
    try {
      await launchApp("pulsar-store", STORE_URL);
    } catch {
      await openUrl(STORE_URL);
    }
  };

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-10 py-6 overflow-y-auto">
          <div className="flex items-center justify-center gap-6">
            <img
              src="./logos/sayri.png"
              alt="Sayri"
              className="h-24 w-24 object-contain"
            />
            <div className="h-12 w-px bg-separator/50" />
            <img
              src="./logos/pulsar-store.png"
              alt="Pulsar Store"
              className="h-24 w-24 object-contain"
            />
          </div>

          <h1 className="mt-5 text-center text-[28px] sm:text-[32px] font-semibold leading-tight text-text-primary">
            Meet Sayri & Pulsar Store
          </h1>
          <p className="mt-3 max-w-140 text-center text-[14px] sm:text-[15px] leading-relaxed text-text-secondary">
            Your intelligent AI assistant and software ecosystem, built directly into Pulsar OS. Sayri executes complex tasks safely with complete permission control, while the Pulsar Store provides apps, skills, plugins, and system extensions.
          </p>

          <div className="mt-6 grid w-full max-w-150 grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <Sparkles className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Skills & Agents
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Natural command agents that run automated workflows safely.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <ShieldCheck className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Permissions & Control
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Every action is sandboxed and under your control.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <ShoppingBag className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Pulsar Store
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Skills, plugins, extensions and curated apps.
              </p>
            </div>
          </div>

          <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
            <button
              className="btn-primary flex items-center gap-2"
              onClick={handleOpenStore}
            >
              <ShoppingBag className="h-4 w-4" />
              <span>Open Pulsar Store</span>
            </button>
            <button
              className="btn-secondary flex items-center gap-2"
              onClick={() => openUrl(STORE_URL)}
            >
              <ExternalLink className="h-4 w-4" />
              <span>Explore Online</span>
            </button>
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