interface WifiNotNetworkProps {
  scan: () => void;
}

export function WifiNotNetwork({ scan }: WifiNotNetworkProps) {
  return (
    <div className="py-10 text-center">
      <p className="text-sm text-text-secondary">No networks found.</p>
      <button
        onClick={scan}
        className="mt-3 text-sm font-medium text-accent hover:underline"
      >
        Scan again
      </button>
    </div>
  );
}
