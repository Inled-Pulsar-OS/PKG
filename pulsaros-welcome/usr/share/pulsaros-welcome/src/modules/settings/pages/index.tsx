import {
  launchApp,
  launchWifiSettings,
  launchBluetoothSettings,
  launchDisplaySettings,
  launchAppearanceSettings,
} from "@/modules/core/api";
import { SETTINGS_CARDS, type SettingsCard } from "../constants";

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
  }
}

export function SettingsPage({ onContinue, onBack }: SettingsPageProps) {
  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[88vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        <header className="flex shrink-0 flex-col items-center px-8 pt-12 pb-2 select-none">
          <h1 className="text-center text-[26px] font-semibold leading-tight text-text-primary sm:text-[30px]">
            Settings you might like
          </h1>
          <p className="mt-2.5 max-w-130 text-center text-[14px] leading-relaxed text-text-secondary sm:text-[15px]">
            Quick access to the most useful settings in Pulsar OS.
          </p>
        </header>

        <main className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 pb-4">
          <div className="m-auto grid w-full max-w-150 grid-cols-1 gap-4 sm:grid-cols-2">
            {SETTINGS_CARDS.map((card) => {
              const Icon = card.icon;
              return (
                <button
                  key={card.id}
                  onClick={() => runAction(card)}
                  className="glass-grouped flex cursor-pointer items-start gap-4 rounded-2xl p-5 text-left transition-shadow hover:shadow-md"
                >
                  <div className="shrink-0 rounded-xl bg-apple-blue/10 p-2.5">
                    <Icon className="h-6 w-6 text-apple-blue" strokeWidth={2} />
                  </div>
                  <div>
                    <div className="text-[15px] font-semibold text-text-primary">
                      {card.title}
                    </div>
                    <p className="mt-1 text-[12px] leading-snug text-text-secondary">
                      {card.description}
                    </p>
                  </div>
                </button>
              );
            })}
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