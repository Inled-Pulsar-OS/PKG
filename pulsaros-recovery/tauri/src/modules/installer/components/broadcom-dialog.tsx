interface BroadcomDialogProps {
  detected: boolean;
  onConfirm: (install: boolean) => void;
}

export function BroadcomDialog({ detected, onConfirm }: BroadcomDialogProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="glass mx-4 w-full max-w-sm rounded-2xl p-6">
        <h3 className="text-[18px] font-semibold text-text-primary">
          Broadcom Drivers
        </h3>
        <p className="mt-2 text-[14px] text-text-secondary">
          {detected
            ? "Broadcom WiFi hardware detected. Install proprietary drivers?"
            : "No Broadcom hardware detected. Skip driver installation?"}
        </p>
        <div className="mt-6 flex justify-end gap-2">
          <button
            className="btn-secondary"
            onClick={() => onConfirm(false)}
          >
            Skip
          </button>
          <button
            className="btn-primary"
            onClick={() => onConfirm(true)}
          >
            {detected ? "Install Drivers" : "Continue Without"}
          </button>
        </div>
      </div>
    </div>
  );
}
