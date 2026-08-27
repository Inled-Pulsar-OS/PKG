import { useState } from "react";
import { Screen } from "@/modules/ui";
import { AvatarPicker } from "../components/avatar-picker";

interface AccountPageProps {
  avatars: string[];
  onContinue: (account: {
    fullName: string;
    username: string;
    password: string;
    avatar: string;
  }) => void;
  onBack: () => void;
}

export function AccountPage({ avatars, onContinue, onBack }: AccountPageProps) {
  const [fullName, setFullName] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [avatar, setAvatar] = useState(avatars[0] ?? "");

  const canContinue = fullName.trim() && username.trim() && password.trim();

  return (
    <Screen
      title="Create Your Account"
      subtitle="Set up your user profile."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button
            className="btn-primary"
            disabled={!canContinue}
            onClick={() =>
              onContinue({ fullName: fullName.trim(), username: username.trim(), password, avatar })
            }
          >
            Continue
          </button>
        </div>
      }
    >
      <div className="flex flex-col items-center gap-5">
        <AvatarPicker avatars={avatars} selected={avatar} onSelect={setAvatar} />

        <input
          type="text"
          placeholder="Full name"
          value={fullName}
          onChange={(e) => {
            setFullName(e.target.value);
            if (!username) setUsername(e.target.value.toLowerCase().replace(/\s+/g, "."));
          }}
          className="h-9 w-full max-w-[300px] rounded-lg border border-border bg-white/60 px-3 text-[13px] text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
        />

        <input
          type="text"
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          className="h-9 w-full max-w-[300px] rounded-lg border border-border bg-white/60 px-3 text-[13px] text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
        />

        <input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className="h-9 w-full max-w-[300px] rounded-lg border border-border bg-white/60 px-3 text-[13px] text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
        />
      </div>
    </Screen>
  );
}
