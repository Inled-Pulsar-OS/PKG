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
        "cursor-pointer flex w-full items-center gap-4 px-4 py-3.5 text-left transition-colors",
        "border-b border-separator last:border-b-0",
        "hover:bg-black/[0.04]",
        selected && "bg-black/[0.06]"
      )}
      onClick={() => onSelect(option.id)}
    >
      <img
        src={option.icon}
        alt=""
        className="h-9 w-9 shrink-0 rounded-lg object-contain"
      />
      <div className="flex min-w-0 flex-col gap-0.5">
        <div className="truncate text-[15px] font-medium text-text-primary">
          {option.title}
        </div>
        <div className="text-[13px] leading-snug text-text-secondary">
          {option.desc}
        </div>
      </div>
    </button>
  );
}
