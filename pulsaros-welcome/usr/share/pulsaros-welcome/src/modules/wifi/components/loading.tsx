export function WifiLoading() {
  return (
    <div className="flex items-center justify-center py-10">
      <div className="h-5 w-5 animate-spin rounded-full border-2 border-text-secondary border-t-transparent" />
      <span className="ml-3 text-sm text-text-secondary">
        Scanning networks...
      </span>
    </div>
  );
}
