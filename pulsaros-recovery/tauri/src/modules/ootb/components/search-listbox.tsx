import { useState, useMemo } from "react";

interface SearchListboxProps {
  items: string[];
  placeholder: string;
  onSelect: (item: string) => void;
  selected?: string;
}

export function SearchListbox({
  items,
  placeholder,
  onSelect,
  selected,
}: SearchListboxProps) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    if (!query) return items;
    const q = query.toLowerCase();
    return items.filter((item) => item.toLowerCase().includes(q));
  }, [items, query]);

  return (
    <div className="flex flex-col items-center gap-3">
      <input
        type="text"
        placeholder={placeholder}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="h-9 w-full max-w-[320px] rounded-lg border border-border bg-white/60 px-3 text-[13px] text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
      />
      <div className="h-[180px] w-full max-w-[320px] overflow-y-auto rounded-lg border border-border bg-white/40">
        {filtered.length === 0 ? (
          <p className="p-3 text-center text-[12px] text-text-secondary">
            No results found
          </p>
        ) : (
          filtered.map((item) => (
            <button
              key={item}
              className={`flex w-full items-center px-3 py-2 text-left text-[13px] transition-colors hover:bg-white/60 ${
                selected === item
                  ? "bg-accent/10 font-medium text-accent"
                  : "text-text-primary"
              }`}
              onClick={() => onSelect(item)}
            >
              {item}
            </button>
          ))
        )}
      </div>
    </div>
  );
}
