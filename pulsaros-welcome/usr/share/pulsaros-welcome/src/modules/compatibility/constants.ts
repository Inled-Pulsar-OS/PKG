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
    id: "macboat",
    name: "MacBoat",
    description:
      "Run macOS in a VM, downloaded from Apple's official servers. OpenCore integration, no hassle.",
    logo: "./logos/macboat.png",
    launch: "macboat",
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
    id: "gsconnect",
    name: "GSConnect",
    description:
      "Notifications, file sharing, and remote control. Your phone and desktop, unified.",
    logo: "./logos/gsconnect.png",
    launch:
      "/usr/share/gnome-shell/extensions/gsconnect@andyholmes.github.io/gsconnect-preferences",
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

export const KDE_CONNECT_QR = "./logos/kdeconnect-qr.png";
export const KDE_CONNECT_URL = "https://kdeconnect.kde.org/download.html";