import { Screen } from "@/modules/ui";
import { SearchListbox } from "../components/search-listbox";

interface LanguagePageProps {
  languages: string[];
  selected: string | null;
  onSelect: (lang: string) => void;
  onContinue: () => void;
  onBack: () => void;
}

export function LanguagePage({
  languages,
  selected,
  onSelect,
  onContinue,
  onBack,
}: LanguagePageProps) {
  return (
    <Screen
      title="Select Language"
      subtitle="Choose your preferred language."
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
        items={languages}
        placeholder="Search languages..."
        selected={selected ?? undefined}
        onSelect={onSelect}
      />
    </Screen>
  );
}
