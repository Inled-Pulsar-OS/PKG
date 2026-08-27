import { RecoveryProvider, useRecoveryContext } from "@/modules/core";
import { UtilitiesPage } from "@/modules/utilities";
import { ReinstallChoicePage } from "@/modules/reinstall-choice";
import { TargetSelectPage } from "@/modules/target-select";
import { ProgressPage } from "@/modules/progress";
import { CompletePage } from "@/modules/complete";
import { ErrorPage } from "@/modules/error";
import { InstallerWelcomePage } from "@/modules/installer";
import { OotbCountryPage } from "@/modules/ootb";

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
      return <InstallerWelcomePage />;
    case "ootb":
      return <OotbCountryPage />;
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
