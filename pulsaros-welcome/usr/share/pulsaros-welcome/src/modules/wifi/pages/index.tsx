import { useEffect, useState, useCallback } from "react";
import { Layout } from "@/modules/ui";
import {
  scanWifiNetworks,
  connectToWifi,
  launchWifiSettings,
} from "@/modules/core/api";
import type { WifiNetwork, ConnectForm } from "@/modules/wifi/types";
import { cn } from "@/modules/ui/utils";
import { WifiLoading } from "../components/loading";
import { WifiNotNetwork } from "../components/not-network";
import { WifiNetworkList } from "../components/network-list";
import { isSecured } from "../utils";
import { INITIAL_FORM } from "../constants";

interface WifiPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function WifiPage({ onContinue, onBack }: WifiPageProps) {
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [loading, setLoading] = useState(true);
  const [form, setForm] = useState<ConnectForm>(INITIAL_FORM);

  const scan = useCallback(async () => {
    setLoading(true);
    const result = await scanWifiNetworks();
    console.log("scan result", result);
    setNetworks(result);
    setLoading(false);
  }, []);

  useEffect(() => {
    scan();
  }, [scan]);

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
            Continue
          </button>
        </div>
      }
    >
      <div className="flex flex-col items-center gap-4">
        {/* <WifiIcon /> */}

        {/* Network list */}
        <div className="w-full overflow-hidden rounded-xl border border-border bg-white/5">
          {loading && <WifiLoading />}
          {networks.length === 0 && !loading && <WifiNotNetwork scan={scan} />}
          {networks.length > 0 && !loading && (
            <div className="max-h-56 overflow-y-auto">
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

        {/* Password field — secured networks only */}
        {form.selected && isSecured(selectedNetwork?.security ?? "") && (
          <div className="w-full max-w-sm">
            <div className="relative">
              <input
                type={form.showPassword ? "text" : "password"}
                value={form.password}
                onChange={(e) =>
                  setForm((f) => ({ ...f, password: e.target.value }))
                }
                placeholder="Password"
                className="w-full rounded-lg border border-border bg-white/5 px-4 py-2.5 pr-10 text-sm text-text-primary placeholder-text-secondary/60 outline-none focus:border-accent focus:ring-1 focus:ring-accent/30"
              />
              <button
                type="button"
                onClick={() =>
                  setForm((f) => ({ ...f, showPassword: !f.showPassword }))
                }
                className="absolute right-3 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
              >
                {form.showPassword ? (
                  <svg viewBox="0 0 20 20" className="h-4 w-4 fill-current">
                    <path d="M10 3C5.6 3 1.7 6.1.3 10c1.4 3.9 5.3 7 9.7 7s8.3-3.1 9.7-7c-1.4-3.9-5.3-7-9.7-7Zm0 11.5a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Zm0-7a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5Z" />
                  </svg>
                ) : (
                  <svg viewBox="0 0 20 20" className="h-4 w-4 fill-current">
                    <path d="M10 3C5.6 3 1.7 6.1.3 10c1.4 3.9 5.3 7 9.7 7s8.3-3.1 9.7-7c-1.4-3.9-5.3-7-9.7-7Zm0 11.5a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Zm0-7a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5Z" />
                  </svg>
                )}
              </button>
            </div>
          </div>
        )}

        {/* Join button */}
        {form.selected && (
          <button
            onClick={handleJoin}
            disabled={form.connecting || form.success}
            className="btn-primary min-w-25"
          >
            {form.connecting ? (
              <span className="flex items-center justify-center gap-2">
                <span className="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent" />
                Joining...
              </span>
            ) : form.success ? (
              "Connected"
            ) : (
              "Join"
            )}
          </button>
        )}

        {/* Error */}
        {form.error && (
          <p className="text-center text-xs text-red-500">{form.error}</p>
        )}

        {/* Fallback — no nmcli */}
        {!loading && networks.length === 0 && (
          <button
            className="btn-secondary mt-2"
            onClick={() => launchWifiSettings()}
          >
            Open Wi-Fi Settings
          </button>
        )}
      </div>
    </Layout>
  );
}
