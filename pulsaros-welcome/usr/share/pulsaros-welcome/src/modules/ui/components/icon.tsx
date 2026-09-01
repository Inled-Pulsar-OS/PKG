import { cn } from "../utils/cn";

const paths: Record<string, string> = {
  clock:
    "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2",
  refresh:
    "M21 12a9 9 0 11-3.36-6.93M21 3v5h-5",
  globe:
    "M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10A15.3 15.3 0 0112 2z",
  disk:
    "M4 7v10c0 1.1.9 2 2 2h12a2 2 0 002-2V7M4 7a2 2 0 012-2h12a2 2 0 012 2M4 7l2-3h12l2 3M12 11v4",
  check:
    "M22 11.08V12a10 10 0 11-5.93-9.14M22 4L12 14.01l-3-3",
  x: "M18 6L6 18M6 6l12 12",
};

export type IconName = keyof typeof paths;

interface IconProps {
  name: IconName;
  className?: string;
}

export function Icon({ name, className }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("inline-block align-middle", className)}
    >
      <path d={paths[name]} />
    </svg>
  );
}
