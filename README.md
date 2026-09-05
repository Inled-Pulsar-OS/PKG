# Pulsar OS – PKG (Package Sources)

This repository holds the package sources that make up **Pulsar OS** and its
NAS-oriented sibling **Tube OS**. Every folder with a `DEBIAN/control` file is a
Debian package; `arch/` additionally carries the Arch Linux `PKGBUILD`s for the
same components. Packages are compiled with `package-and-deploy.sh` (locally or
in GitHub Actions) into `.deb` files that are published to the Inled central APT
repository and consumed by the [ISO build](https://github.com/Inled-Pulsar-OS/ISO).

> `pulsaros-meta` is a metapackage that depends on every Pulsar OS package.
> Installing it on an existing installation pulls in anything new that has been
> added since the system was installed.

---

## What each program does

### Branding, identity & content

| Package | What it does |
|---|---|
| `pulsaros-branding` | Brands the base OS as **Pulsar OS "Bitten Fruit"**: ships `/etc/os-release`, `/etc/issue`, hostname conventions and system identity files. |
| `pulsaros-emoji-fonts` | Installs **Google Noto Color Emoji**, the standard emoji font of Linux desktops, so every application gets full color-emoji support. |
| `pulsar-pear-sound-theme` | macOS-like **system sound theme**: alerts, notifications, volume changes, device connection, trash and dialog sounds for Linux desktop environments. |
| `pulsaros-bootsound` | Plays the official **startup sound** at boot with `aplay` through a systemd service. |
| `pulsar-boot-icons` | High-resolution **bootloader icons** (live and installed boot entries) used by the rEFInd and GRUB themes. |
| `pulsaros-essential` | System glue for the desktop & live environment: ships the **`pulsar-store`** CLI and the **Fildem HUD** global-menu framework, configures locales, network interfaces, the official wallpapers (registered in GNOME), passwordless sudo/polkit for the `live` user, NVIDIA VRAM-preservation modprobe options, systemd sleep configuration for hibernation and the default `mimeapps` (Seafari, AppInstall…). |

### Boot & login

| Package | What it does |
|---|---|
| `pulsaros-plymouth` | macOS-like **Plymouth boot splash**, replacing the default Debian/Arch boot logos. |
| `pulsaros-grub` | macOS-like **GRUB theme** (bootloader menu), packaged and configured for BIOS/EFI installs. |
| `pulsaros-refind` | **rEFInd** configuration with a macOS-style theme, packaged from the Inled rEFInd theme repository. |
| `pulsaros-sddm` | Makes **SDDM** the default display manager using the **Apple Tahoe** login theme (macOS Tahoe look, video-wallpaper capable). Keeps the gdm3 libraries so GNOME screen locking keeps working. |
| `pulsaros-hibernate` | Hibernation UX: shows a **Plymouth progress splash** during hibernation (covering the NVIDIA VRAM save and the kernel-image write) and a **resume-offset guardian** that verifies at every boot that the rEFInd `resume_offset` matches the real `/swapfile` geometry, repairing it automatically if the swapfile is recreated or moved. |

### Desktop & GNOME (the macOS-like experience)

| Package | What it does |
|---|---|
| `pulsaros-theme` | **MacTahoe GTK theme** (GTK3/GTK4) and **MacTahoe icon theme**, compiled and applied system-wide and to new users via `/etc/skel`. |
| `pulsaros-gnome` | The Pulsar desktop extension layer: downloads and preinstalls a curated stack of GNOME Shell extensions, compiles and installs **Pulsar Dock** (the Inled Dash-to-Dock fork) and **Liquid Glass**, and injects GSchema overrides + dconf databases that produce the macOS-like desktop out of the box. Replaces `gnome-shell-extension-dashtodock`. |
| `pulsaros-global-menu` | Large in-house GNOME Shell extension (GNOME 45–50, Wayland): **Mac-style global application menus** in the top bar (Apple menu, File/Edit/View…, power-off/restart dialogs with a 60 s countdown) plus a full-screen **custom lock screen** that can play the live video wallpaper. |
| `pulsaros-control-center` | Custom compile of **GNOME Control Center** (48.x) with Pulsar OS patches: Appearance panel with the Live Wallpaper engine, Liquid Glass effect, macOS-style accent colors and custom layout. Provides/replaces the stock `gnome-control-center`. |
| `pulsaros-control-center-button` | Small GNOME Shell extension that adds a **macOS-style Control Center button** to the Quick Settings / system menu, opening GNOME Control Center. |
| `pulsaros-effects-settings` | Desktop-effects manager that toggles between **Blur my Shell** and **Liquid Glass** glassmorphism and re-configures Dash-to-Dock for optimal rendering. |
| `gnome-macos-remap-wayland` | **macOS keyboard remapping for GNOME on Wayland** built on `xremap`: ships the remap config and offline `xremap` binaries (GNOME/X11), patched so the macOS shortcut scheme composes with the XKB Ctrl↔Super swap that Pulsar OS applies via dconf. |
| `pulsaros-spotlight-launcher` | **Spotlight-like search**: a native Rust/GTK4 search window with full-text file & application search powered by GNOME Tracker/TinySPARQL (`localsearch`), helper CLIs (`pulsaros-toggle-remap`, `pulsaros-toggle-launcher`) and a GNOME Shell extension adding the Spotlight icon to the top bar. Replaces the older `spotlight-gtk` / `spotlight-python` packages. |
| `nautilus` | **Finder**, the Pulsar OS file manager: a custom build of GNOME Files with a macOS-inspired layout, real-time **folder color tagging** and a per-color filter in the sidebar. Compiled from the Inled Finder fork (a git submodule) and ABI-compatible with stock Nautilus. |
| `pulsaros-live-wallpaper` | **Animated / video wallpaper engine and daemon**: plays MP4, WebM, GIF, MKV and WebP wallpapers natively through the Hidamari engine, and synchronizes a poster frame plus the same wallpaper across the GNOME desktop background, the lock screen and the SDDM login screen. |
| `pulsaros-cloud` | **Cloud drives in the Finder sidebar**: a wrapper around `rclone` that adds, mounts and removes cloud accounts (Google Drive, OneDrive, iCloud or any rclone backend) under `~/Cloud/<name>`, with a systemd user mount unit. |

### Apps, assistants & utilities

| Package | What it does |
|---|---|
| `pulsaros-welcome` | First-boot **Welcome / OOTB** application: a native Tauri (Rust + React) app that detects live / first-boot / installed systems, shows the Apple-like "hello" animation, a feature slideshow, app-compatibility info, settings, and shortcuts to Sayri and Recovery, then closes by itself. Ships the OOTB and user-cleanup helpers (`pulsaros-ootb`, `pulsar-cleanup-user`). |
| `pulsaros-recovery` | **Recovery & installer UI** in the style of macOS Recovery: a full-screen GTK4/Libadwaita flow with Recovery Utilities (Install Pulsar OS, Seafari browser, Disk Utility/GParted, Time Machine, extra packages), disk/partition pickers, clean-install and dual-boot installers writing Btrfs with rEFInd/GRUB, plus a native Rust helper (`pulsar-recovery-assistant`) for privileged operations. |
| `pulsaros-timemachine` | **Continuous backup suite**: local **Btrfs snapshots** that are synced with **restic** to USB drives, Samba/NAS shares and rclone cloud storage. Ships a GTK4/Libadwaita UI, a scheduler service and a CLI. |
| `pulsaros-circle-to-search` | **Circle to Search** for the Linux desktop: shake the mouse or press a shortcut, select a screen region, and get real-time **OCR** (Tesseract), an optional AI query answered by **Sayri**, and instant visual Google search. GNOME Shell extension + Python helper. |
| `sayri` | **Siri-like AI assistant**: an always-available orb pinned to the top of the screen (GTK4 layer-shell). Speech-to-text with **whisper.cpp**, text-to-speech with **Piper**, and any OpenAI-compatible provider (OpenAI, Ollama, LM Studio…). Runs in agent mode with 5 sandbox levels, wake word, skills, plugins and gateway instances (Discord, Telegram…). |
| `driverman` | **GPU driver manager**: detects the installed GPU and manages its driver packages through `apt`, with a CLI (`driverman`) and a dark GTK front-end (`driverman-gui`). If a package install hits a dependency conflict the user keeps control of a terminal to resolve it. |
| `scrcpy` | The **scrcpy** Android screen mirroring & control utility, packaged for Pulsar OS (uses `adb`). |

### Tube OS (NAS / home-server line)

| Package | What it does |
|---|---|
| `tube-os-dash` | **Tube OS Dashboard**: the elegant web dashboard and app orchestrator of Tube OS — Docker app store/management, storage (mergerfs, Samba, NTFS…), disks/health (smartmontools) and cloud mounts (rclone). |
| `dockermigrate` | **Docker migration tool with a web UI**: export and import Docker containers, volumes and compose projects between machines; web interface on port 8070 for selecting containers and managing backups (systemd service included). |
| `tubeos-installer` | **Headless web installer / OOTB setup** for Tube OS: a browser-accessible (FastAPI) installer at `http://tubeos.local` that installs Tube OS to disk and configures timezone, keyboard and user. No HDMI needed — just ethernet + a browser. |
| `tubeos-branding` | Logos, icons and branding assets for the Tube OS distribution. |
| `tubeos-plymouth` | Animated **Plymouth boot splash** for Tube OS using the Tube OS logo. |

### Support folders & scripts

| Path | What it is |
|---|---|
| `arch/` | `PKGBUILD`s that package the same Pulsar OS components for the **Arch Linux** edition of the ISO. |
| `apple-hello/` | Source of the Apple-style "hello" **boot animation** used by the Welcome app (a CodePen original by *steef*). |
| `pulsaros-control-center-button/` | Standalone sources of the "Control Center Button" GNOME Shell extension. |
| `.github/workflows/` | CI that compiles the packages (`build-packages.yml`) and deploys them to the Inled APT repository. |
| `package-and-deploy.sh` | Builds one, several or all packages into `.deb`s and (optionally) uploads + dispatches them to the central APT repo. |
| `install-local.sh` / `install-spotlight-local.sh` | Local-development installers (global menu / Spotlight extension). |
| `test-*.sh`, `check-repo-versions.sh`, `test-installer-mounts.py` | Testing and repo-health helpers. |
| `build/` | Local build staging (`.deb` output), not source. |

---

## Derivatives

"Derivatives" lists which upstream, third-party or in-house project every
program derives from or is built around. Themes and extensions that Inled
maintains in its own repositories are noted as such.

### Forks & upstream derivatives

| Package | Derives from / built around |
|---|---|
| `nautilus` (Finder) | **GNOME Files / Nautilus** (GNOME Project) — custom macOS-inspired fork, GPL-3.0. Built from the Inled fork repo (git submodule `Inled-Pulsar-OS/finder`). |
| `pulsaros-control-center` | **GNOME Control Center** (GNOME Project) — recompiled from upstream GNOME source with Pulsar OS patches (same overlay as the Arch edition). |
| `pulsaros-circle-to-search` | **Shotzy** by *SamkitJain660* (EGO extension #9707) — fork that adds the Sayri AI query box, Google Lens/visual search and screenshot uploader. Keeps Shotzy's GPL-3.0 license. |
| `pulsaros-gnome` → `pulsar-dock@inled.es` | **Dash-to-Dock** (micheleg/dash-to-dock) — Inled fork maintained at `Inled-Pulsar-OS/dash-to-dock`. |
| `pulsaros-gnome` → `liquid-glass@…` | **Liquid Glass** GNOME extension by *thinkingcoding1231* — Inled fork maintained at `InledGroup/liquid-glass`. |
| `pulsaros-gnome` (other extensions) | Bundles selected EGO extensions as-is: **Blur my Shell** (aunetx), **Wiggle** (mechtifs, patched for GNOME 50), **Compiz alike magic lamp effect** (hermes83), **Just Perfection**, **Notification Banner Position** (drugo), **No overview** (fthx), **Desktop Icons NG / ding** (rastersoft), **GSConnect** (andyholmes), **GNOME UI Tune** (itstime.tech), **AppIndicator & KStatusNotifierItem** (rgcjonas) and **User Themes**. |
| `pulsaros-essential` | **Fildem HUD** (*gonzaarcr/Fildem*, the Unity-HUD-style global menu) — Inled fork at `InledGroup/Fildem`, shipped with Pulsar OS patches. |
| `gnome-macos-remap-wayland` | **gnome-macos-remap** project config (Inled fork repo) on top of the **xremap** key-remapper (`xremap/xremap`). |
| `pulsaros-live-wallpaper` | Built around the **Hidamari** video-wallpaper engine (`io.github.jeffshee.Hidamari`), extended with Pulsar's own daemon and GNOME/SDDM/lock-screen sync. |
| `pulsaros-sddm` → `Apple.Tahoe` theme | **Sonoma-SDDMT** by *zayronxio*, with components/credits to *varlesh* (rounded), *joshuakraemer* (sddm-theme-dialog), *surajmandalcell* (Elegant-sddm) and the SDDM team. |
| `pulsaros-theme` | **MacTahoe GTK + icon themes**, maintained by Inled (`Inled-Pulsar-OS/MacTahoe-gtk-theme`, `MacTahoe-icon-theme`). |
| `pulsaros-grub` | **macOS-like GRUB theme**, maintained by Inled (`Inled-Pulsar-OS/grub.theme`). |
| `pulsaros-refind` | **macOS rEFInd theme**, maintained by Inled (`Inled-Pulsar-OS/refind-mac-theme`), applied on top of **rEFInd** (rEFInd project). |
| `pulsaros-plymouth` | **macOS-like Plymouth theme**, maintained by Inled (`Inled-Pulsar-OS/plymouth-macoslike`), on **Plymouth** (Canonical/Red Hat boot splash). |
| `tube-os-dash` | **CasaOS** by *IceWhale* — rebranded/adapted as the Tube OS dashboard (UI and backend are CasaOS-based; its app store pulls from the **Inled CasaOS app store** and the **big-bear-casaos** store). |
| `pulsaros-spotlight-launcher` | Original **Rust/GTK4** application and extension; search backend is **GNOME Tracker / TinySPARQL** (`localsearch`). Concept mirrors macOS Spotlight. |
| `pulsaros-cloud` | Wrapper around **rclone** (rclone project) — original UI/sidebar integration. |
| `pulsaros-timemachine` | Original Pulsar OS application; engine built on **Btrfs** snapshots + **restic** + **rclone**. Concept mirrors macOS Time Machine. |
| `pulsaros-hibernate` | Original Pulsar OS logic on **Plymouth** and **systemd**; also keeps the NVIDIA hibernate requirements in mind. |
| `pulsaros-welcome` | Original **Tauri (Rust + React)** application. Its "hello" animation is the **apple-hello** CodePen original by *steef* (see below). |
| `sayri` | Original **Python/GTK4** assistant by Inled. Speech stack: **whisper.cpp** (STT) and **Piper** (TTS); designed as a lighter, sandboxed alternative to OpenClaw-style agents for any OpenAI-compatible API. |
| `pulsaros-global-menu`, `pulsaros-control-center-button`, `pulsaros-effects-settings`, `pulsaros-recovery`, `driverman`, `dockermigrate`, `tubeos-installer`, `tubeos-plymouth`, `tubeos-branding`, `pulsaros-branding`, `pulsaros-bootsound`, `pulsar-pear-sound-theme`, `pulsar-boot-icons`, `pulsaros-emoji-fonts`, `pulsaros-meta` | **Original Inled / Pulsar OS work** — no third-party upstream (content may be inspired by macOS visuals, but the code is original). |
| `scrcpy` | **scrcpy** (Genymobile) — repackaged as-is for Pulsar OS. |
| `apple-hello/` | The Apple-style "hello" animation by **steef** (CodePen: steefmaster/MWvdyGb), MIT — the same animation the Welcome app embeds. |

---

## License

All **original code** written by Inled for Pulsar OS / Tube OS is licensed under
the **MIT license (Inled)** — see [license.inled.es](https://license.inled.es).

Programs that are forks or derivatives of third-party projects keep the license
of their upstream, as required by it. Notable examples:

- `nautilus` (Finder) — derived from GNOME Files, **GPL-3.0**.
- `pulsaros-circle-to-search` — derived from Shotzy, **GPL-3.0**.
- `pulsaros-gnome` extensions — each keeps its own upstream license (GPL-2/3,
  MIT…), e.g. Dash-to-Dock and Liquid Glass forks.
- `pulsaros-sddm`'s `Apple.Tahoe` theme — permissive ISC-style license (©
  Alexey Varfolomeev / varlesh).
- `apple-hello` — **MIT** (© steef, CodePen).
- `pulsaros-spotlight-launcher`, `sayri` and the rest of the in-house apps —
  original Inled code, **MIT-INLED**.

When in doubt, read the `LICENSE` / `DEBIAN/copyright` files shipped inside each
package folder.
