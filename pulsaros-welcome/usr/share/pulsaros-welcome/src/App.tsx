import { useState } from "react";
import { HelloPage } from "@/modules/hello";
import { FeaturesPage } from "@/modules/features";
import { closeWindow, writeSentinel } from "@/modules/core/api";

function App() {
  const [screen, setScreen] = useState<"hello" | "features">("hello");

  const handleFinish = async () => {
    try {
      await writeSentinel();
    } catch {
      // ignore
    }
    try {
      await closeWindow();
    } catch {
      window.close();
    }
  };

  if (screen === "hello") {
    return <HelloPage onContinue={() => setScreen("features")} />;
  }

  return (
    <FeaturesPage
      onBack={() => setScreen("hello")}
      onFinish={handleFinish}
    />
  );
}

export default App;
