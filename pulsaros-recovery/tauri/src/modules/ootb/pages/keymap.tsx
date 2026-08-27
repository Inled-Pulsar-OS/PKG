import { Screen } from "@/modules/ui";
import { SearchListbox } from "../components/search-listbox";

interface KeymapPageProps {
  keymaps: string[];
  selected: string | null;
  onSelect: (keymap: string) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function KeymapPage({
  keymaps,
  selected,
  onSelect,
  onContinue,
  onBack,
}: KeymapPageProps) {
  return (
    <Screen
      title="Select Keyboard Layout"
      subtitle="Choose your keyboard layout."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button
            className="btn-primary"
            disabled={!selected}
            onClick={onContinue}
          >
            Continue
          </button>
        </div>
      }
    >
      <SearchListbox
        items={keymaps}
        placeholder="Search layouts..."
        selected={selected ?? undefined}
        onSelect={onSelect}
      />
    </Screen>
  );
}
