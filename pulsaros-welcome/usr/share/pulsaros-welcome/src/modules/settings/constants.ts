import {
  Wifi,
  MonitorCog,
  Bluetooth,
  Cpu,
  Palette,
  ShoppingBag,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export interface SettingsCard {
  id: string;
  title: string;
  description: string;
  icon: LucideIcon;
  action:
    | "wifi"
    | "driverman"
    | "display"
    | "bluetooth"
    | "software"
    | "appearance";
}

export const SETTINGS_CARDS: SettingsCard[] = [
  {
    id: "wifi",
    title: "Wi-Fi",
    description: "Connect to networks and manage connections.",
    icon: Wifi,
    action: "wifi",
  },
  {
    id: "driverman",
    title: "Driver Manager",
    description: "Install, switch or remove GPU drivers.",
    icon: Cpu,
    action: "driverman",
  },
  {
    id: "display",
    title: "Display",
    description: "Resolution, scaling and monitors.",
    icon: MonitorCog,
    action: "display",
  },
  {
    id: "bluetooth",
    title: "Bluetooth",
    description: "Pair headphones, keyboards and mice.",
    icon: Bluetooth,
    action: "bluetooth",
  },
  {
    id: "software",
    title: "Software Center",
    description: "Browse and install applications.",
    icon: ShoppingBag,
    action: "software",
  },
  {
    id: "appearance",
    title: "Appearance",
    description: "Wallpapers, themes and effects.",
    icon: Palette,
    action: "appearance",
  },
];