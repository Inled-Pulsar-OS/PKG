import { Screen, Icon } from "@/modules/ui";
import type { BtrfsTarget } from "@/modules/core/types";

interface InstallerDiskSelectPageProps {
  targets: BtrfsTarget[];
  selectedTarget: BtrfsTarget | null;
  onSelect: (target: BtrfsTarget) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function InstallerDiskSelectPage({
  targets,
  selectedTarget,
  onSelect,
  onContinue,
  onBack,
}: InstallerDiskSelectPageProps) {
  return (
    <Screen
      title="Select Target Disk"
      subtitle="Choose the disk where Pulsar OS will be installed. All data on this disk will be erased."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button
            className="btn-primary"
            disabled={!selectedTarget}
            onClick={onContinue}
          >
            Install
          </button>
        </div>
      }
    >
      {targets.length === 0 ? (
        <p className="py-8 text-center text-[14px] text-text-secondary">
          No disks detected.
        </p>
      ) : (
        <div className="flex flex-wrap justify-center gap-3">
          {targets.map((t) => (
            <button
              key={t.part_path}
              className={`flex min-w-35 flex-col items-center gap-1.5 rounded-xl border bg-white/60 p-4 transition-all hover:bg-white/80 ${
                selectedTarget?.part_path === t.part_path
                  ? "border-accent/40 ring-2 ring-accent/20 bg-white/80"
                  : "border-border"
              }`}
              onClick={() => onSelect(t)}
            >
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/10">
                <Icon name="disk" className="h-5 w-5 text-accent" />
              </div>
              <div className="text-[13px] font-medium text-text-primary">
                {t.label} ({t.size})
              </div>
              <div className="text-[11px] text-text-secondary">
                {t.part_path}
              </div>
            </button>
          ))}
        </div>
      )}
    </Screen>
  );
}
