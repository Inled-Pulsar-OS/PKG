import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { ProgressBar } from "../components/progress-bar";

export function ProgressPage() {
  const { progress, statusText } = useRecoveryContext();

  return (
    <Screen title="">
      <ProgressBar progress={progress} statusText={statusText} />
    </Screen>
  );
}
