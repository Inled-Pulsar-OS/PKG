import { Screen } from "@/modules/ui";
import { SearchListbox } from "../components/search-listbox";

interface TimezonePageProps {
  timezones: string[];
  selected: string | null;
  onSelect: (tz: string) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function TimezonePage({
  timezones,
  selected,
  onSelect,
  onContinue,
  onBack,
}: TimezonePageProps) {
  return (
    <Screen
      title="Select Timezone"
      subtitle="Choose your timezone."
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
        items={timezones}
        placeholder="Search timezones..."
        selected={selected ?? undefined}
        onSelect={onSelect}
      />
    </Screen>
  );
}
