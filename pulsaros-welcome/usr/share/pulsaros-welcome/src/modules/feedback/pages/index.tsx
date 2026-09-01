import { Layout } from "@/modules/ui";
import { open } from "@tauri-apps/plugin-shell";
import { LINKS_HELP } from "../constants";

interface FeedbackPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function FeedbackPage({ onContinue, onBack }: FeedbackPageProps) {
  return (
    <Layout
      title="Beta Feedback & Support"
      subtitle="Pulsar OS is in active development. Help us improve stability by reporting installation bugs, hardware issues or user interface feedback directly to our official issue tracker on GitHub."
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            Start
          </button>
        </div>
      }
    >
      <div className="flex flex-col gap-2">
        {LINKS_HELP.map((link) => (
          <button
            key={link.url}
            className="btn-secondary text-left"
            onClick={() => open(link.url)}
          >
            {link.label}
          </button>
        ))}
      </div>
    </Layout>
  );
}
