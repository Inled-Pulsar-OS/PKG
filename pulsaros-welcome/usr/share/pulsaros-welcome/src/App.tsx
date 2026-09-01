import { useEffect } from "react";
import { Provider, useWelcomeContext } from "@/modules/core";
import { HelloPage } from "@/modules/hello";
import { IntroPage } from "@/modules/intro";
import { ResolutionPage } from "@/modules/resolution";
import { WifiPage } from "@/modules/wifi";
import { BluetoothPage } from "@/modules/bluetooth";
import { SoftwarePage } from "@/modules/software";
import { EffectsPage } from "@/modules/effects";
import { GpuPage } from "@/modules/gpu";
import { FeedbackPage } from "@/modules/feedback";
import { FeaturesPage } from "@/modules/features";
import { CompletePage } from "@/modules/complete";

function WelcomeApp() {
  const {
    screen,
    isLive,
    effectsState,
    resolutions,
    loadSystemInfo,
    goNext,
    goBack,
    setEffects,
    complete,
  } = useWelcomeContext();

  useEffect(() => {
    loadSystemInfo();
  }, [loadSystemInfo]);

  switch (screen) {
    case "hello":
      return <HelloPage onContinue={goNext} />;
    case "intro":
      return <IntroPage isLive={isLive} onContinue={goNext} />;
    case "resolution":
      return (
        <ResolutionPage
          resolutions={resolutions}
          onContinue={goNext}
          onBack={goBack}
        />
      );
    case "wifi":
      return <WifiPage onContinue={goNext} onBack={goBack} />;
    case "bluetooth":
      return <BluetoothPage onContinue={goNext} onBack={goBack} />;
    case "software":
      return <SoftwarePage onContinue={goNext} onBack={goBack} />;
    case "effects":
      return (
        <EffectsPage
          effectsState={effectsState}
          onSetEffects={setEffects}
          onContinue={goNext}
          onBack={goBack}
        />
      );
    case "gpu":
      return <GpuPage onContinue={goNext} onBack={goBack} />;
    case "feedback":
      return <FeedbackPage onContinue={goNext} onBack={goBack} />;
    case "features":
      return <FeaturesPage onContinue={complete} onBack={goBack} />;
    case "done":
      return <CompletePage />;
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
