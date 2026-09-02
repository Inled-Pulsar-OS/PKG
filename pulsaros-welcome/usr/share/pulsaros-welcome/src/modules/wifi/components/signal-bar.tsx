export function SignalBars({ signal }: { signal: number }) {
  const bars = signal >= 80 ? 4 : signal >= 60 ? 3 : signal >= 40 ? 2 : 1;
  console.log("signal", signal, "bars", bars);
  return (
    <svg viewBox="0 0 20 16" className="h-4 w-5 shrink-0" fill="none">
      {[1, 2, 3, 4].map((i) => (
        <rect
          key={i}
          x={i * 4 - 2}
          y={16 - i * 3.5}
          width="3"
          rx="0.5"
          className={i <= bars ? "fill-text-primary" : "fill-black/15"}
        />
      ))}
    </svg>
  );
}
