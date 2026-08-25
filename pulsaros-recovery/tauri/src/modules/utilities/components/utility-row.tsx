import { cn } from "@/modules/ui/utils/cn";
import { Icon } from "@/modules/ui";
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
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent">
        <Icon name={option.icon} className="h-4 w-4 text-white" />
      </div>
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
