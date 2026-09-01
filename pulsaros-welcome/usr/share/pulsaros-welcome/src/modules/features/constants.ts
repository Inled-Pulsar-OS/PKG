export interface ProviderLog {
  name: string;
  src: string;
}

export interface FeatureSlide {
  id: string;
  title: string;
  subtitle: string;
  video?: string;
  providers?: ProviderLog[];
}

export const FEATURE_SLIDES: FeatureSlide[] = [
  {
    id: "session-restore",
    title: "Session Restore",
    subtitle:
      "Everything comes back exactly as you left it. Reboot or power on, and every app, window and document reopens on its own, just like on a Mac.",
    video: "./videos/session-restore.mp4",
  },
  {
    id: "sayri",
    title: "Sayri, AI Assistant",
    subtitle:
      "An AI assistant that uses the model you want. It's not like Siri, it's smart.",
    video: "./videos/sayri.mp4",
  },
  {
    id: "finder-providers",
    title: "Finder & 55 Cloud Providers",
    subtitle:
      "A true Finder clone with Mac-style navigation, previews, tags and sidebar. Up to 55 cloud storage providers supported natively.",
    providers: [
      { name: "Finder", src: "./logos/providers/finder.png" },
      { name: "Google Drive", src: "./logos/providers/googledrive.svg" },
      { name: "Dropbox", src: "./logos/providers/dropbox.svg" },
      { name: "OneDrive", src: "./logos/providers/onedrive.svg" },
      { name: "iCloud", src: "./logos/providers/icloud.svg" },
      { name: "Box", src: "./logos/providers/box.svg" },
      { name: "MEGA", src: "./logos/providers/mega.svg" },
      { name: "Nextcloud", src: "./logos/providers/nextcloud.svg" },
      { name: "Proton", src: "./logos/providers/proton.svg" },
    ],
  },
  {
    id: "app-store",
    title: "App Store",
    subtitle:
      "Browse and install apps from all the package managers. Update your system, uninstall ANY system package and clean system trash.",
    video: "./videos/app-store.mp4",
  },
  {
    id: "spotlight",
    title: "Spotlight",
    subtitle:
      "Search apps, documents, clipboard, images and any file. Navigate dirs and uninstall apps.",
    video: "./videos/spotlight.mp4",
  },
  {
    id: "window-mode",
    title: "Window Mode",
    subtitle:
      "Full screen on new workspace. Full macOS tiling, floating, and split-view window management.",
    video: "./videos/window-mode.mp4",
  },
  {
    id: "remap-wallpaper",
    title: "Remap & Live Wallpapers",
    subtitle:
      "Switch between Mac or Linux shortcuts in one click, and enjoy animated wallpapers on the Desktop and SDDM.",
    video: "./videos/remap-live-wallpaper.mp4",
  },
];