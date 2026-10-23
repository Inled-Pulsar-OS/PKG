export interface AppEntry {
  id: string;
  name: string;
  description: string;
  logo: string;
  launch: string;
  fallback?: string;
  storeUrl?: string;
}

export const CROSS_PLATFORM_APPS: AppEntry[] = [
  {
    id: "pulsar-phone",
    name: "Pulsar Phone",
    description:
      "Seamlessly connect your Android phone. Stream screens, run mobile apps on PC and sync notifications.",
    logo: "./logos/pulsar-phone.svg",
    launch: "pulsar-vpn",
  },
  {
    id: "sayri",
    name: "Sayri AI",
    description:
      "Your intelligent native assistant with voice, vision and deep system integration.",
    logo: "./logos/sayri.png",
    launch: "sayri",
  },
  {
    id: "winboat",
    name: "WinBoat",
    description:
      "Windows apps as native windows. Real Windows instance underneath, no Wine, no activation.",
    logo: "./logos/winboat.svg",
    launch: "winboat",
  },
  {
    id: "droidtux",
    name: "DroidTux",
    description:
      "Android apps run as desktop windows. Your phone's apps, on your big screen.",
    logo: "./logos/droidtux.png",
    launch: "droidtux-sync",
    fallback: "droidtux-settings",
  },
];