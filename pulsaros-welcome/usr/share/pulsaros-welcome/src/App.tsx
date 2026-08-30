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
      return <FeedbackPage onContinue={complete} onBack={goBack} />;
    case "done":
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
              Setup Complete
            </h2>
            <p className="mt-2 text-[14px] text-text-secondary">
              Pulsar OS has been configured successfully.
            </p>
          </div>
        </div>
      );
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
