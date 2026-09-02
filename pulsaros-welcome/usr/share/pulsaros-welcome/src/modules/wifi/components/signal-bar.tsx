import {
  Signal,
  SignalHigh,
  SignalLow,
  SignalMedium,
  SignalZero,
} from "lucide-react";

const VARIANTS = [
  { min: 0, Icon: SignalZero },
  { min: 20, Icon: SignalLow },
  { min: 40, Icon: SignalMedium },
  { min: 60, Icon: SignalHigh },
  { min: 80, Icon: Signal },
];

export function SignalBars({ signal }: { signal: number }) {
  const { Icon } = VARIANTS.reduce((match, v) => (signal >= v.min ? v : match));
  return <Icon size={16} className="shrink-0 text-text-primary" />;
}
