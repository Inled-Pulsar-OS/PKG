import { useEffect, useState } from "react";
import {
  launchApp,
  launchWifiSettings,
  launchBluetoothSettings,
  launchDisplaySettings,
  launchAppearanceSettings,
  launchOptimizerGui,
  getDarkMode,
  setDarkMode,
  getGlobalMenuState,
  setGlobalMenu,
  getBootsoundState,
  setBootsoundState,
  getOptimizerState,
  setOptimizerState,
} from "@/modules/core/api";
import { SETTINGS_CARDS, type SettingsCard } from "../constants";
import { Moon, Sun, Menu, Zap, Volume2 } from "lucide-react";

interface SettingsPageProps {
  onContinue: () => void;
  onBack: () => void;
}

function runAction(card: SettingsCard) {
  switch (card.action) {
    case "wifi":
      launchWifiSettings();
      break;
    case "driverman":
      launchApp("driverman-gui", "driverman");
      break;
    case "display":
      launchDisplaySettings();
      break;
    case "appearance":
      launchAppearanceSettings();
      break;
    case "bluetooth":
      launchBluetoothSettings();
      break;
    case "software":
      launchApp("appinstall");
      break;
    case "optimizer":
      launchOptimizerGui();
      break;
  }
}

export function SettingsPage({ onContinue, onBack }: SettingsPageProps) {
  const [isDark, setIsDark] = useState<boolean>(true);
  const [globalMenu, setGlobalMenuState] = useState<boolean>(true);
  const [bootsound, setBootsound] = useState<boolean>(true);
  const [optimizer, setOptimizer] = useState<boolean>(true);

  useEffect(() => {
    getDarkMode().then(setIsDark).catch(() => {});
    getGlobalMenuState().then(setGlobalMenuState).catch(() => {});
    getBootsoundState().then(setBootsound).catch(() => {});
    getOptimizerState().then(setOptimizer).catch(() => {});
  }, []);

  const handleDarkModeToggle = async () => {
    const next = !isDark;
    setIsDark(next);
    await setDarkMode(next).catch(() => {});
  };

  const handleGlobalMenuToggle = async () => {
    const next = !globalMenu;
    setGlobalMenuState(next);
    await setGlobalMenu(next).catch(() => {});
  };

  const handleBootsoundToggle = async () => {
    const next = !bootsound;
    setBootsound(next);
    await setBootsoundState(next).catch(() => {});
  };

  const handleOptimizerToggle = async () => {
    const next = !optimizer;
    setOptimizer(next);
    await setOptimizerState(next).catch(() => {});
  };

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-4 sm:p-6">
      <div className="screen-enter glass flex h-[90vh] max-h-[780px] w-full max-w-[880px] flex-col overflow-hidden">
        <header className="flex shrink-0 flex-col items-center px-8 pt-8 pb-2 select-none">
          <h1 className="text-center text-[24px] font-semibold leading-tight text-text-primary sm:text-[28px]">
            System Preferences & Customization
          </h1>
          <p className="mt-2 max-w-140 text-center text-[13px] leading-relaxed text-text-secondary sm:text-[14px]">
            Configure essential features, customize appearance, and launch system utilities.
          </p>
        </header>

        <main className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-3 sm:px-8 space-y-5">
          {/* Section 1: Live Interactive Feature Toggles */}
          <div>
            <h2 className="mb-2 text-[12px] font-semibold uppercase tracking-wider text-text-secondary">
              Quick System Features
            </h2>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              {/* Dark / Light Mode */}
              <div className="glass-grouped flex items-center justify-between rounded-xl p-3.5">
                <div className="flex items-center gap-3">
                  <div className="rounded-lg bg-apple-blue/10 p-2 text-apple-blue">
                    {isDark ? <Moon className="h-5 w-5" /> : <Sun className="h-5 w-5" />}
                  </div>
                  <div>
                    <div className="text-[14px] font-medium text-text-primary">
                      {isDark ? "Dark Mode" : "Light Mode"}
                    </div>
                    <div className="text-[11px] text-text-secondary">
                      MacTahoe dynamic theme scheme
                    </div>
                  </div>
                </div>
                <button
                  onClick={handleDarkModeToggle}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                    isDark ? "bg-apple-blue" : "bg-neutral-600"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                      isDark ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>

              {/* Real Global Menu */}
              <div className="glass-grouped flex items-center justify-between rounded-xl p-3.5">
                <div className="flex items-center gap-3">
                  <div className="rounded-lg bg-emerald-500/10 p-2 text-emerald-400">
                    <Menu className="h-5 w-5" />
                  </div>
                  <div>
                    <div className="text-[14px] font-medium text-text-primary">
                      Real Global Menu
                    </div>
                    <div className="text-[11px] text-text-secondary">
                      macOS top bar menu for all apps
                    </div>
                  </div>
                </div>
                <button
                  onClick={handleGlobalMenuToggle}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                    globalMenu ? "bg-apple-blue" : "bg-neutral-600"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                      globalMenu ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>

              {/* Dynamic Optimizer Service */}
              <div className="glass-grouped flex items-center justify-between rounded-xl p-3.5">
                <div className="flex items-center gap-3">
                  <div className="rounded-lg bg-amber-500/10 p-2 text-amber-400">
                    <Zap className="h-5 w-5" />
                  </div>
                  <div>
                    <div className="text-[14px] font-medium text-text-primary">
                      Zero-Lag Optimizer
                    </div>
                    <div className="text-[11px] text-text-secondary">
                      Adaptive load & habit governor
                    </div>
                  </div>
                </div>
                <button
                  onClick={handleOptimizerToggle}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                    optimizer ? "bg-apple-blue" : "bg-neutral-600"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                      optimizer ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>

              {/* Startup Boot Sound */}
              <div className="glass-grouped flex items-center justify-between rounded-xl p-3.5">
                <div className="flex items-center gap-3">
                  <div className="rounded-lg bg-purple-500/10 p-2 text-purple-400">
                    <Volume2 className="h-5 w-5" />
                  </div>
                  <div>
                    <div className="text-[14px] font-medium text-text-primary">
                      Startup Boot Sound
                    </div>
                    <div className="text-[11px] text-text-secondary">
                      Pulsar OS chime on system startup
                    </div>
                  </div>
                </div>
                <button
                  onClick={handleBootsoundToggle}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                    bootsound ? "bg-apple-blue" : "bg-neutral-600"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                      bootsound ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>
            </div>
          </div>

          {/* Section 2: Application & Utility Launchers */}
          <div>
            <h2 className="mb-2 text-[12px] font-semibold uppercase tracking-wider text-text-secondary">
              Applications & Hardware Control
            </h2>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              {SETTINGS_CARDS.map((card) => {
                const Icon = card.icon;
                return (
                  <button
                    key={card.id}
                    onClick={() => runAction(card)}
                    className="glass-grouped flex cursor-pointer items-start gap-3.5 rounded-xl p-4 text-left transition-all hover:bg-white/10"
                  >
                    <div className="shrink-0 rounded-lg bg-apple-blue/10 p-2 text-apple-blue">
                      <Icon className="h-5 w-5" strokeWidth={2} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="text-[14px] font-semibold text-text-primary">
                        {card.title}
                      </div>
                      <p className="mt-0.5 text-[11px] leading-snug text-text-secondary line-clamp-1">
                        {card.description}
                      </p>
                    </div>
                  </button>
                );
              })}
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