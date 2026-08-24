import { RecoveryProvider, useRecoveryContext } from "@/modules/core";
import { UtilitiesPage } from "@/modules/utilities";
import { TargetSelectPage } from "@/modules/target-select";
import { ProgressPage } from "@/modules/progress";
import { CompletePage } from "@/modules/complete";
import { ErrorPage } from "@/modules/error";

function CurrentScreen() {
  const { screen } = useRecoveryContext();

  switch (screen) {
    case "utilities":
      return <UtilitiesPage />;
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

function App() {
  return (
    <RecoveryProvider>
      <CurrentScreen />
    </RecoveryProvider>
  );
}

export default App;
