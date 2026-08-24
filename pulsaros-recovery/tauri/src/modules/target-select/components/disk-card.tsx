import { cn } from "@/modules/ui/utils/cn";
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
        "flex min-w-[140px] flex-col items-center gap-1 rounded-xl border border-[#3c3c3c] bg-[#2a2a2a] p-5 transition-all hover:bg-[#323236]",
        selected && "border-accent shadow-[0_0_0_2px_var(--color-accent)] bg-[#323236]"
      )}
      onClick={() => onSelect(target)}
    >
      <div className="text-[36px]">💾</div>
      <div className="text-[13px] font-semibold text-white">
        {target.label} ({target.size})
      </div>
      <div className="text-[11px] text-text-secondary">{target.part_path}</div>
    </button>
  );
}
