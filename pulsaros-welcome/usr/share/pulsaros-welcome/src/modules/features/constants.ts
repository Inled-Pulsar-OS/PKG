import { FeatureSlide } from "./types";

export const SLIDE_MS = 10000;

export const FEATURE_SLIDES: FeatureSlide[] = [
  {
    id: "session-restore",
    title: "Session Restore",
    subtitle:
      "Everything comes back exactly as you left it. Reboot or power on, and every app, window and document reopens on its own, just like on a Mac.",
    video: "./videos/session-restore.webp",
  },
  {
    id: "flydrop",
    title: "Send files from any OS, like with Apple",
    subtitle:
      "Flydrop is like AirDrop but on Pulsar OS. Use it on your devices with Localsend apps",
  },
  {
    id: "optimizer",
    title: "Your PC is not very fast?",
    subtitle:
      "Thanks to Pulsar Optimizer, your computer will no longer freeze every minute! Reassign priority to each application and process for a seamless experience.",
  },
  {
    id: "global-menu",
    title: "Real Global Menu",
    subtitle:
      "No Linux distribution offers a real global menu like MacOS, with the real menus of almost all apps.",
  },
  {
    id: "app-store",
    title: "AppInstall is the app store",
    subtitle:
      "For anything related to installing something, Appinstall offers you a friendly visual interface similar to the style of the MacOS app store. You won't have to use the terminal",
    video: "./videos/app-store.webp",
  },
  {
    id: "sayri",
    title: "Sayri, AI Assistant",
    subtitle:
      "An AI assistant that uses the model you want. It's not like Siri, it's smart.",
    video: "./videos/sayri.webp",
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
    id: "spotlight",
    title: "Spotlight",
    subtitle:
      "Search apps, documents, clipboard, images and any file. Navigate dirs and uninstall apps.",
    video: "./videos/spotlight.webp",
  },
  {
    id: "window-mode",
    title: "Window Mode",
    subtitle:
      "Full screen on new workspace. Full macOS tiling, floating, and split-view window management.",
    video: "./videos/window-mode.webp",
  },
  {
    id: "remap-wallpaper",
    title: "Remap & Live Wallpapers",
    subtitle:
      "Switch between Mac or Linux shortcuts in one click, and enjoy animated wallpapers on the Desktop and SDDM.",
    video: "./videos/remap-live-wallpaper.webp",
  },
  {
    id: "adblock",
    title: "Never more ads with our system-level AdBlock!",
    subtitle:
      "Say goodbye to the damn ads that MacOS, Windows or other Linux distros don't protect you from! Welcome to tranquility!",
  },
];