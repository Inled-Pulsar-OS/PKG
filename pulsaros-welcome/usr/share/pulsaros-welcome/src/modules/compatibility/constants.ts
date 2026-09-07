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
    id: "install-macos",
    name: "Install macOS",
    description:
      "Run and manage macOS virtual machines on KVM with native speed and simple setup.",
    logo: "./logos/install-macos.png",
    launch: "install-macos",
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