import { useEffect } from "react";
import { RecoveryProvider, useRecoveryContext } from "@/modules/core";
import { UtilitiesPage } from "@/modules/utilities";
import { ReinstallChoicePage } from "@/modules/reinstall-choice";
import { TargetSelectPage } from "@/modules/target-select";
import { ProgressPage } from "@/modules/progress";
import { CompletePage } from "@/modules/complete";
import { ErrorPage } from "@/modules/error";
import {
  InstallerProvider,
  useInstallerContext,
} from "@/modules/installer";
import { InstallerWelcomePage } from "@/modules/installer/pages/welcome";
import { InstallerDiskSelectPage } from "@/modules/installer/pages/disk-select";
import { InstallerProgressPage } from "@/modules/installer/pages/progress";
import { InstallerErrorPage } from "@/modules/installer/pages/error";
import { BroadcomDialog } from "@/modules/installer/components/broadcom-dialog";
import { OotbProvider, useOotbContext } from "@/modules/ootb";
import { CountryPage } from "@/modules/ootb/pages/country";
import { LanguagePage } from "@/modules/ootb/pages/language";
import { KeymapPage } from "@/modules/ootb/pages/keymap";
import { TimezonePage } from "@/modules/ootb/pages/timezone";
import { AccountPage } from "@/modules/ootb/pages/account";
import { OotbProgressPage } from "@/modules/ootb/pages/progress";
import { FinishedPage } from "@/modules/ootb/pages/finished";

function RecoveryScreen() {
  const { screen } = useRecoveryContext();

  switch (screen) {
    case "utilities":
      return <UtilitiesPage />;
    case "reinstall_choice":
      return <ReinstallChoicePage />;
    case "target_select":
      return <TargetSelectPage />;
    case "progress":
      return <ProgressPage />;
    case "complete":
      return <CompletePage />;
    case "error":
      return <ErrorPage />;
  }
}

function InstallerApp() {
  const {
    screen,
    targets,
    selectedTarget,
    broadcomDetected,
    progress,
    statusText,
    error,
    logs,
    setSelectedTarget,
    goToDiskSelect,
    goToBroadcom,
    startInstallation,
    goBack,
    goBackFromError,
  } = useInstallerContext();

  switch (screen) {
    case "welcome":
      return <InstallerWelcomePage onContinue={goToDiskSelect} />;
    case "disk_select":
      return (
        <InstallerDiskSelectPage
          targets={targets}
          selectedTarget={selectedTarget}
          onSelect={setSelectedTarget}
          onContinue={goToBroadcom}
          onBack={goBack}
        />
      );
    case "broadcom":
      return (
        <>
          <InstallerDiskSelectPage
            targets={targets}
            selectedTarget={selectedTarget}
            onSelect={setSelectedTarget}
            onContinue={goToBroadcom}
            onBack={goBack}
          />
          <BroadcomDialog
            detected={broadcomDetected}
            onConfirm={(install) => startInstallation(install)}
          />
        </>
      );
    case "progress":
      return (
        <InstallerProgressPage
          progress={progress}
          statusText={statusText}
          logs={logs}
        />
      );
    case "complete":
      return (
        <div className="bg-atmosphere flex h-screen w-screen items-center justify-center p-5 sm:p-8">
          <div className="glass flex h-[65vh] max-h-175 min-h-50 w-[60vw] max-w-200 min-w-125 flex-col items-center justify-center overflow-hidden">
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-green-500/10">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="h-8 w-8 text-green-500"
              >
                <path d="M22 11.08V12a10 10 0 11-5.93-9.14M22 4L12 14.01l-3-3" />
              </svg>
            </div>
            <h2 className="mt-4 text-[22px] font-semibold text-text-primary">
              Installation Complete
            </h2>
            <p className="mt-2 text-[14px] text-text-secondary">
              Pulsar OS has been installed successfully.
            </p>
          </div>
        </div>
      );
    case "error":
      return (
        <InstallerErrorPage
          error={error}
          onBack={goBackFromError}
          onRetry={goToDiskSelect}
        />
      );
  }
}

function OotbApp() {
  const {
    screen,
    countries,
    languages,
    keymaps,
    timezones,
    avatars,
    selectedCountry,
    selectedLanguage,
    selectedKeymap,
    selectedTimezone,
    progress,
    statusText,
    logs,
    loadData,
    setSelectedCountry,
    setSelectedLanguage,
    setSelectedKeymap,
    setSelectedTimezone,
    goToLanguage,
    goToKeymap,
    goToTimezone,
    goToAccount,
    goBackToCountry,
    goBackToLanguage,
    goBackToKeymap,
    goBackToTimezone,
    runSetup,
    reboot,
  } = useOotbContext();

  useEffect(() => {
    loadData();
  }, [loadData]);

  switch (screen) {
    case "country":
      return (
        <CountryPage
          countries={countries}
          selected={selectedCountry}
          onSelect={setSelectedCountry}
          onContinue={goToLanguage}
        />
      );
    case "language":
      return (
        <LanguagePage
          languages={languages}
          selected={selectedLanguage}
          onSelect={setSelectedLanguage}
          onContinue={goToKeymap}
          onBack={goBackToCountry}
        />
      );
    case "keymap":
      return (
        <KeymapPage
          keymaps={keymaps}
          selected={selectedKeymap}
          onSelect={setSelectedKeymap}
          onContinue={goToTimezone}
          onBack={goBackToLanguage}
        />
      );
    case "timezone":
      return (
        <TimezonePage
          timezones={timezones}
          selected={selectedTimezone}
          onSelect={setSelectedTimezone}
          onContinue={goToAccount}
          onBack={goBackToKeymap}
        />
      );
    case "account":
      return (
        <AccountPage
          avatars={avatars}
          onContinue={runSetup}
          onBack={goBackToTimezone}
        />
      );
    case "progress":
      return (
        <OotbProgressPage
          progress={progress}
          statusText={statusText}
          logs={logs}
        />
      );
    case "finished":
      return <FinishedPage onReboot={reboot} />;
    case "error":
      return (
        <div className="bg-atmosphere flex h-screen w-screen items-center justify-center p-5 sm:p-8">
          <div className="glass flex h-[65vh] max-h-175 min-h-50 w-[60vw] max-w-200 min-w-125 flex-col items-center justify-center overflow-hidden">
            <h2 className="text-[22px] font-semibold text-red-500">
              Setup Failed
            </h2>
            <p className="mt-2 text-[14px] text-text-secondary">
              An error occurred during setup.
            </p>
          </div>
        </div>
      );
  }
}

function CurrentScreen() {
  const { mode } = useRecoveryContext();

  if (mode === null) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#1c1c1e] text-white">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-white border-t-transparent" />
      </div>
    );
  }

  switch (mode) {
    case "installer":
      return (
        <InstallerProvider>
          <InstallerApp />
        </InstallerProvider>
      );
    case "ootb":
      return (
        <OotbProvider>
          <OotbApp />
        </OotbProvider>
      );
    case "recovery":
    default:
      return <RecoveryScreen />;
  }
}

function App() {
  return (
    <RecoveryProvider>
      <CurrentScreen />
    </RecoveryProvider>
  );
}

export default App;
