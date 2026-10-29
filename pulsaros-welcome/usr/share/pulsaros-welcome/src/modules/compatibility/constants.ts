export interface AppEntry {
  id: string;
  name: string;
  description: string;
  logo: string;
  launch: string;
  fallback?: string;
  /** Optional second button that opens a URL (e.g. the Android app download page). */
  url?: string;
  urlLabel?: string;
}

/** Every Pulsar OS Android app is published in the official Inled F-Droid repo. */
export const FDROID_REPO = "https://fdroid.inled.es";

export const CROSS_PLATFORM_APPS: AppEntry[] = [
  {
    id: "pulsar-windows",
    name: "Pulsar Windows",
    description:
      "Windows apps as native windows. Real Windows instance underneath, no Wine, no activation.",
    logo: "./logos/pulsar-windows.svg",
    launch: "pulsar-windows",
  },
  {
    id: "pulsar-phone",
    name: "Pulsar Phone",
    description:
      "Seamlessly connect your Android phone. Stream screens, run mobile apps on PC and sync notifications.",
    logo: "./logos/pulsar-phone.svg",
    launch: "pulsar-vpn",
    url: FDROID_REPO,
    urlLabel: "Get the Android app",
  },
  {
    id: "sayri",
    name: "Sayri AI",
    description:
      "Your intelligent native assistant with voice, vision and deep system integration. Sayri Mobile runs it on your phone.",
    logo: "./logos/sayri.png",
    launch: "sayri",
    url: FDROID_REPO,
    urlLabel: "Get Sayri for Android",
  },
];
