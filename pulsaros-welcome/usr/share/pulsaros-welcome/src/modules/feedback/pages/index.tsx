import { Layout } from "@/modules/ui";
import { open } from "@tauri-apps/plugin-shell";

interface FeedbackPageProps {
  onContinue: () => void;
  onBack: () => void;
}

const links = [
  {
    label: "Open Issues Page on GitHub",
    url: "https://github.com/Inled-Pulsar-OS/ISO/issues",
  },
  {
    label: "Open Official Wiki",
    url: "https://github.com/Inled-Pulsar-OS/DOCS/wiki",
  },
  {
    label: "Visit Official Website",
    url: "https://os.inled.es",
  },
  {
    label: "Join Discord",
    url: "https://discord.gg/PSeTkDMnr",
  },
];

export function FeedbackPage({ onContinue, onBack }: FeedbackPageProps) {
  return (
    <Layout
      title="Beta Feedback & Support"
      subtitle="Pulsar OS is in active development. Help us improve stability by reporting installation bugs, hardware issues or user interface feedback directly to our official issue tracker on GitHub."
      footer={
        <div className="flex w-full items-center justify-between">
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
        {links.map((link) => (
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
