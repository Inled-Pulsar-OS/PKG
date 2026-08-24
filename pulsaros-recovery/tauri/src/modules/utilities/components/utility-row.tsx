import { cn } from "@/modules/ui/utils/cn";
import type { UtilityOption } from "../data/utilities";

interface UtilityRowProps {
  option: UtilityOption;
  selected: boolean;
  onSelect: (id: UtilityOption["id"]) => void;
}

export function UtilityRow({ option, selected, onSelect }: UtilityRowProps) {
  return (
    <button
      className={cn(
        "flex w-full items-center gap-4 px-[18px] py-3.5 text-left transition-colors",
        "border-b border-white/[0.06] last:border-b-0",
        "hover:bg-row-hover",
        selected && "bg-row-selected"
      )}
      onClick={() => onSelect(option.id)}
    >
      <div className="shrink-0 text-[32px]">{option.icon}</div>
      <div className="flex flex-col gap-0.5">
        <div className="text-[15px] font-semibold text-white">{option.title}</div>
        <div className="text-[13px] text-text-secondary">{option.desc}</div>
      </div>
    </button>
  );
}
