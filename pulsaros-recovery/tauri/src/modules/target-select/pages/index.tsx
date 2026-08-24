import { Screen } from "@/modules/ui";
import { useRecoveryContext } from "@/modules/core";
import { DiskCard } from "../components/disk-card";

export function TargetSelectPage() {
  const { targets, selectedTarget, setSelectedTarget, startRestoreFlow, goBack } =
    useRecoveryContext();

  return (
    <Screen
      title="Select Pulsar OS Partition"
      subtitle="The root system (@) will be cleanly restored. Your documents, applications, and settings in /home (@home) will remain intact."
    >
      {targets.length === 0 ? (
        <p className="text-center text-[13px] text-text-secondary">
          No Btrfs Pulsar OS partitions detected.
          <br />
          Use Disk Utility to inspect drives.
        </p>
      ) : (
        <div className="mb-4 flex w-full flex-wrap justify-center gap-3">
          {targets.map((t) => (
            <DiskCard
              key={t.part_path}
              target={t}
              selected={selectedTarget?.part_path === t.part_path}
              onSelect={setSelectedTarget}
            />
          ))}
        </div>
      )}
      <div className="mt-auto flex w-full gap-3 pt-4">
        <button
          className="rounded-lg border border-white/[0.15] bg-[#323236] px-6 py-2.5 text-[14px] font-semibold text-white transition-colors hover:bg-[#3e3e42]"
          onClick={goBack}
        >
          Back
        </button>
        <button
          className="rounded-lg bg-accent px-6 py-2.5 text-[14px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:bg-[#38383a] disabled:text-[#636366] disabled:cursor-default"
          disabled={!selectedTarget}
          onClick={startRestoreFlow}
        >
          Restore System
        </button>
      </div>
    </Screen>
  );
}
