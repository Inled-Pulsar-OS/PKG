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
    | "appearance"
    | "optimizer";
}

export const SETTINGS_CARDS: SettingsCard[] = [
  {
    id: "software",
    title: "AppInstall",
    description: "Unified App Center for apps, packages & extensions.",
    icon: ShoppingBag,
    action: "software",
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
    id: "appearance",
    title: "Appearance",
    description: "Wallpapers, themes and accent colors.",
    icon: Palette,
    action: "appearance",
  },
  {
    id: "wifi",
    title: "Wi-Fi",
    description: "Connect to networks and manage connections.",
    icon: Wifi,
    action: "wifi",
  },
  {
    id: "bluetooth",
    title: "Bluetooth",
    description: "Pair headphones, keyboards and mice.",
    icon: Bluetooth,
    action: "bluetooth",
  },
];