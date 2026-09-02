import { Eye, EyeOff } from "lucide-react";
import type { WifiNetwork, ConnectForm } from "../types";
import { isSecured } from "../utils";
import { SignalBars } from "./signal-bar";

interface WifiDetailPanelProps {
  network: WifiNetwork | undefined;
  form: ConnectForm;
  setForm: (form: React.SetStateAction<ConnectForm>) => void;
  onJoin: () => void;
}

export function WifiDetailPanel({
  network,
  form,
  setForm,
  onJoin,
}: WifiDetailPanelProps) {
  if (!network) return null;

  const secured = isSecured(network.security);

  return (
    <div className="rounded-xl border border-border bg-white/5 p-4">
      {/* Network header */}
      <div className="flex items-center gap-2.5">
        <SignalBars signal={network.signal} />
        <span className="text-sm font-medium text-text-primary">
          {network.ssid}
        </span>
      </div>

      {/* Security type + Band */}
      <div className="mt-1.5 flex items-center gap-1.5 pl-6.5 text-xs text-text-secondary">
        {secured && <span>{network.security}</span>}
        {secured && network.band && <span>·</span>}
        {network.band && <span>{network.band}</span>}
      </div>

      {/* Rate */}
      {network.rate && (
        <p className="mt-0.5 pl-6.5 text-xs text-text-secondary">
          Up to {network.rate}
        </p>
      )}

      {/* Password field + Show password */}
      {secured && (
        <div className="mt-4">
          <div className="relative">
            <input
              type={form.showPassword ? "text" : "password"}
              value={form.password}
              onChange={(e) =>
                setForm((f) => ({ ...f, password: e.target.value }))
              }
              placeholder="Password"
              className="w-full rounded-lg border border-border bg-white/5 px-3 py-2 pr-9 text-sm text-text-primary placeholder-text-secondary/60 outline-none focus:border-accent focus:ring-1 focus:ring-accent/30"
            />
            <button
              type="button"
              onClick={() =>
                setForm((f) => ({ ...f, showPassword: !f.showPassword }))
              }
              className="cursor-pointer absolute right-2.5 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
            >
              {form.showPassword ? (
                <EyeOff className="size-4" />
              ) : (
                <Eye className="size-4" />
              )}
            </button>
          </div>
        </div>
      )}

      {/* Join button */}
      <button
        onClick={onJoin}
        disabled={form.connecting || form.success}
        className="btn-primary mt-4 w-full"
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

      {/* Error */}
      {form.error && (
        <p className="mt-2 text-center text-xs text-red-500">{form.error}</p>
      )}
    </div>
  );
}
