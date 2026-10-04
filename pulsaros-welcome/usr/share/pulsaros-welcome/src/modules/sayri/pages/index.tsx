import { Sparkles, ShieldCheck, Cpu, Bot, Settings } from "lucide-react";
import { launchApp } from "@/modules/core/api";

interface SayriPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function SayriPage({ onContinue, onBack }: SayriPageProps) {
  const handleLaunchSayri = async () => {
    try {
      await launchApp("sayri", "es.inled.Sayri");
    } catch {
      // ignore
    }
  };

  const handleOpenSayriSettings = async () => {
    try {
      await launchApp("sayri-settings", "sayri");
    } catch {
      // ignore
    }
  };

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-10 py-6 overflow-y-auto">
          <div className="flex items-center justify-center">
            <img
              src="./logos/sayri.png"
              alt="Sayri"
              className="h-28 w-28 object-contain drop-shadow-lg"
            />
          </div>

          <h1 className="mt-5 text-center text-[28px] sm:text-[32px] font-semibold leading-tight text-text-primary">
            Sayri, your Personal AI Assistant
          </h1>
          <p className="mt-3 max-w-140 text-center text-[14px] sm:text-[15px] leading-relaxed text-text-secondary">
            Sayri brings intelligent natural language workflows, multi-modal reasoning, and system task automation directly into Pulsar OS. Choose any model, execute skills safely, and stay in total control.
          </p>

          <div className="mt-6 grid w-full max-w-150 grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <Sparkles className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Skills & Agents
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Natural command agents that automate system workflows safely.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <ShieldCheck className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Permissions & Control
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Every action is sandboxed with explicit security authorization.
              </p>
            </div>
            <div className="glass-grouped flex flex-col items-center rounded-2xl p-4 text-center">
              <Cpu className="mb-2 h-7 w-7 text-apple-blue" strokeWidth={2} />
              <div className="text-[13px] font-semibold text-text-primary">
                Model Choice
              </div>
              <p className="mt-1 text-[11.5px] leading-snug text-text-secondary">
                Use local offline neural models or cloud AI providers of your choice.
              </p>
            </div>
          </div>

          <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
            <button
              className="btn-primary flex items-center gap-2"
              onClick={handleLaunchSayri}
            >
              <Bot className="h-4 w-4" />
              <span>Launch Sayri</span>
            </button>
            <button
              className="btn-secondary flex items-center gap-2"
              onClick={handleOpenSayriSettings}
            >
              <Settings className="h-4 w-4" />
              <span>Configure AI</span>
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