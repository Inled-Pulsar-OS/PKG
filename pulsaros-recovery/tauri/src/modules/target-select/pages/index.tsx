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
      footer={
        <>
          <button className="btn-secondary" onClick={goBack}>
            Back
          </button>
          <button
            className="btn-primary"
            disabled={!selectedTarget}
            onClick={startRestoreFlow}
          >
            Restore System
          </button>
        </>
      }
    >
      {targets.length === 0 ? (
        <p className="py-8 text-center text-[14px] text-text-secondary">
          No Btrfs Pulsar OS partitions detected.
          <br />
          Use Disk Utility to inspect drives.
        </p>
      ) : (
        <div className="flex flex-wrap justify-center gap-3">
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
    </Screen>
  );
}
