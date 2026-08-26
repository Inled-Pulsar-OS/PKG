import { cn } from "@/modules/ui/utils/cn";
import { Icon } from "@/modules/ui";
import type { BtrfsTarget } from "@/modules/core/types";

interface DiskCardProps {
  target: BtrfsTarget;
  selected: boolean;
  onSelect: (target: BtrfsTarget) => void;
}

export function DiskCard({ target, selected, onSelect }: DiskCardProps) {
  return (
    <button
      className={cn(
        "flex min-w-35 flex-col items-center gap-1.5 rounded-xl border border-border bg-white/60 p-4 transition-all",
        "hover:bg-white/80",
        selected && "border-accent/40 ring-2 ring-accent/20 bg-white/80",
      )}
      onClick={() => onSelect(target)}
    >
      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/10">
        <Icon name="disk" className="h-5 w-5 text-accent" />
      </div>
      <div className="text-[13px] font-medium text-text-primary">
        {target.label} ({target.size})
      </div>
      <div className="text-[11px] text-text-secondary">{target.part_path}</div>
    </button>
  );
}
