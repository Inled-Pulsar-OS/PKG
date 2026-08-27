interface AvatarPickerProps {
  avatars: string[];
  selected: string;
  onSelect: (path: string) => void;
}

export function AvatarPicker({ avatars, selected, onSelect }: AvatarPickerProps) {
  return (
    <div className="flex flex-wrap justify-center gap-3">
      {avatars.map((avatar) => (
        <button
          key={avatar}
          className={`flex h-15 w-15 items-center justify-center overflow-hidden rounded-full border-2 transition-all ${
            selected === avatar
              ? "border-accent ring-2 ring-accent/20"
              : "border-border hover:border-accent/40"
          }`}
          onClick={() => onSelect(avatar)}
        >
          {avatar.startsWith("/") ? (
            <img
              src={`asset://localhost/${avatar}`}
              alt="Avatar"
              className="h-full w-full object-cover"
            />
          ) : (
            <span className="text-2xl">{avatar}</span>
          )}
        </button>
      ))}
    </div>
  );
}
