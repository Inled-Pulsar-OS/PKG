import { useEffect } from "react";
import { Provider, useWelcomeContext } from "@/modules/core";
import { HelloPage } from "@/modules/hello";
import { FeaturesPage } from "@/modules/features";
import { CompatibilityPage } from "@/modules/compatibility";
import { SettingsPage } from "@/modules/settings";
import { SayriPage } from "@/modules/sayri";
import { RecoveryPage } from "@/modules/recovery";
import { DonePage } from "@/modules/done";
import { WifiPage } from "@/modules/wifi";

function WelcomeApp() {
  const {
    screen,
    loadSystemInfo,
    proceedFromHello,
    goNext,
    goBack,
  } = useWelcomeContext();

  useEffect(() => {
    loadSystemInfo();
  }, [loadSystemInfo]);

  switch (screen) {
    case "hello":
      return <HelloPage onContinue={proceedFromHello} />;
    case "features":
      return <FeaturesPage onContinue={goNext} onBack={goBack} />;
    case "compatibility":
      return <CompatibilityPage onContinue={goNext} onBack={goBack} />;
    case "settings":
      return <SettingsPage onContinue={goNext} onBack={goBack} />;
    case "sayri":
      return <SayriPage onContinue={goNext} onBack={goBack} />;
    case "recovery":
      return <RecoveryPage onContinue={goNext} onBack={goBack} />;
    case "wifi":
      return <WifiPage onContinue={goNext} onBack={goBack} />;
    case "done":
      return <DonePage />;
  }
}

function App() {
  return (
    <Provider>
      <WelcomeApp />
    </Provider>
  );
}

export default App;