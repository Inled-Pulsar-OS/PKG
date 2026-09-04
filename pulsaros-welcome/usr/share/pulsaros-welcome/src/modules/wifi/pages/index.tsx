import { useState, useCallback, useEffect } from "react";
import { Layout } from "@/modules/ui";
import {
  scanWifiNetworks,
  connectToWifi,
  launchWifiSettings,
} from "@/modules/core/api";
import type { WifiNetwork, ConnectForm } from "@/modules/wifi/types";
import { WifiLoading } from "../components/loading";
import { WifiNotNetwork } from "../components/not-network";
import { WifiNetworkList } from "../components/network-list";
import { isSecured } from "../utils";
import { INITIAL_FORM } from "../constants";
import { WifiDetailPanel } from "../components/network-detail-panel";
import { cn } from "@/modules/ui/utils";

interface WifiPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function WifiPage({ onContinue, onBack }: WifiPageProps) {
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState<ConnectForm>(INITIAL_FORM);

  const scan = useCallback(async () => {
    setLoading(true);
    const result = await scanWifiNetworks();
    setNetworks(result);
    setLoading(false);
    const connected = result.find((n) => n.in_use);
    if (connected) {
      setForm((f) => (f.selected ? f : { ...INITIAL_FORM, selected: connected.ssid }));
    }
  }, []);

  const selectedNetwork = networks.find((n) => n.ssid === form.selected);

  const handleJoin = async () => {
    if (!form.selected) return;
    setForm((f) => ({ ...f, connecting: true, error: null }));

    const secured = isSecured(selectedNetwork?.security ?? "");
    const result = await connectToWifi(
      form.selected,
      secured ? form.password : undefined,
    );

    if (result === true || result === undefined) {
      setForm((f) => ({ ...f, connecting: false, success: true }));
      setTimeout(onContinue, 1500);
    } else {
      setForm((f) => ({
        ...f,
        connecting: false,
        error: typeof result === "string" ? result : "Connection failed.",
      }));
    }
  };

  const handleSelect = (ssid: string) => {
    setForm({ ...INITIAL_FORM, selected: ssid });
  };

  useEffect(() => {
    scan();
  }, [scan]);

  return (
    <Layout
      title="Select Your Wi-Fi Network"
      subtitle="Choose a network to connect to the internet."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            {!loading && !networks.length ? "Skip" : "Continue"}
          </button>
        </div>
      }
    >
      <div className="flex items-center w-full gap-4">
        {loading && (
          <div className="flex-1 overflow-hidden rounded-xl border border-border bg-white/5">
            <WifiLoading />
          </div>
        )}
        {/* Left: Network list */}
        <div
          className={cn(
            "flex flex-col items-center gap-2",
            !loading && !networks.length && "w-full",
            !form.selected && networks.length > 0 && "w-full",
          )}
        >
          <div className="overflow-hidden rounded-xl border border-border bg-white/5 w-full">
            {!networks.length && !loading && <WifiNotNetwork scan={scan} />}
            {networks.length > 0 && !loading && (
              <div className="max-h-100 overflow-y-auto w-full">
                {networks.map((n) => (
                  <WifiNetworkList
                    key={n.ssid}
                    network={n}
                    networkSelected={form.selected}
                    handleSelect={handleSelect}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Scan again */}
          {!loading && networks.length > 0 && (
            <button
              onClick={scan}
              className="text-xs text-text-secondary hover:text-text-primary hover:underline"
            >
              Scan again
            </button>
          )}
        </div>

        {/* Right: Detail panel */}
        {form.selected && (
          <div className="flex-1">
            <WifiDetailPanel
              network={selectedNetwork}
              form={form}
              setForm={setForm}
              onJoin={handleJoin}
            />
          </div>
        )}
      </div>

      {/* Fallback — no nmcli */}
      {!loading && !networks.length && (
        <button
          className="btn-secondary mt-2 self-center"
          onClick={() => launchWifiSettings()}
        >
          Open Wi-Fi Settings
        </button>
      )}
    </Layout>
  );
}
