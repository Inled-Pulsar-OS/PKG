# Pulsar OS – PKG (Package Sources)  

> [!IMPORTANT]
> **Pulsar OS 2.0 Package Status & Deprecations**  
> Este repositorio contiene el código fuente de los paquetes para **Pulsar OS** (Bitten Fruit) y su versión para servidores NAS **Tube OS**. A continuación se detallan los paquetes actuales del sistema, los nuevos componentes introducidos en la versión 2.0 y los paquetes formalmente **deprecados / descontinuados** que ya no deben incluirse en nuevas imágenes ni instalaciones.

Este repositorio almacena el código de los paquetes de Pulsar OS, los cuales son desarrollados tanto para Debian como Arch.  
Estos paquetes incluyen un archivo de `prepare-assets.sh` cuya tarea es colocar los assets en el staging para que luego se empaqueten para sistema Arch o Debian. Los paquetes se pueden compilar e instalar en local (para el proceso de desarrollo) pero para desplegarse a los usuarios siempre se empaquetan desde GH Actions, garantizando la reproducibilidad y confianza. Los paquetes son desplegados al repositorio [APT Inled](https://apt.inled.es) (llamado apt pero realmente es un repositorio pacman, dnf y apt). Estos paquetes son luego utilizados para la creación de la ISO (también desde GH Actions) así como para proveer de las nuevas funcionalidades a los usuarios que tienen el sistema ya instalado.  

Este repositorio contiene todos los paquetes para todas las distribuciones de Pulsar OS, las cuales actualmente son Bittenfruit y Tube OS.  

Posiblemente quiera [consultar el repositorio ISO](https://github.com/Inled-Pulsar-OS/ISO) para conocer cómo se construye la imagen ISO.  

---

## ⚠️ Deprecated & Discontinued Packages / Paquetes obsoletos

Los siguientes paquetes están marcados como **obsoletos** o han sido completamente **descontinuados** en Pulsar OS 2.0:

| Package | Status | Razón / Reemplazo |
|---|---|---|
| `pulsar-store` | **Descontinuado / Eliminado** | La tienda anterior de Pulsar Store ha sido retirada y no se incluye en la ISO ni en la instalación. La gestión de paquetes y software se realiza mediante las herramientas nativas del sistema, Flatpak y AppInstall. |
| `dockermigrate` | **Obsoleto** | Herramienta experimental previa para migración de contenedores Docker en Tube OS; descontinuada en favor de las herramientas nativas de copia y restauración de volúmenes de CasaOS/Docker. |
| `pulsaros-control-center-button` | **Obsoleto** | Sustituido por la integración nativa en GNOME Shell a través de `pulsaros-control-center`, `Pulsar-CC-27-Gnome` y `Pulsar-Notification-Center`. |
| `gnome-macos-remap-wayland` | **Obsoleto** | Reasignación basada en xremap descontinuada. En Pulsar OS 2.0 la reasignación de teclas (Ctrl↔Super/Command, accesos de teclado macOS) se gestiona de forma nativa a través de GNOME Mutter y las opciones de XKB en dconf. |
| `driverman` | **Obsoleto** | Asistente de controladores antiguo de Debian; sustituido por la detección automática en el entorno de instalación y recuperación de `pulsaros-recovery` y los repositorios oficiales. |
| `scrcpy` | **Obsoleto (Empaquetado)** | Empaquetado interno legado; se utiliza directamente el paquete upstream oficial provisto por los repositorios de Arch / Debian. |
| `hblock` | **Obsoleto** | Paquete binario redistribuido reemplazado por la solución integrada moderna `pulsaros-hblock` y el cortafuegos `pulsaros-firewall`. |

---

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
| `pulsaros-essential` | System glue for the desktop & live environment: ships default locales, network interfaces, official wallpapers (registered in GNOME), passwordless sudo/polkit for the `live` user, NVIDIA VRAM-preservation modprobe options, systemd sleep configuration for hibernation and default MIME associations. |

### Boot & login

| Package | What it does |
|---|---|
| `pulsaros-plymouth` | macOS-like **Plymouth boot splash**, replacing the default Debian/Arch boot logos. |
| `pulsaros-grub` | macOS-like **GRUB theme** (bootloader menu), packaged and configured for BIOS/EFI installs. |
| `pulsaros-refind` | **rEFInd** configuration with a macOS-style theme, packaged from the Inled rEFInd theme repository with single-label clean icons. |
| `pulsaros-sddm` | Makes **SDDM** the default display manager using the **Apple Tahoe** login theme (macOS Tahoe look, video-wallpaper capable). Keeps the gdm3 libraries so GNOME screen locking keeps working. |
| `pulsaros-hibernate` | Hibernation UX: shows a **Plymouth progress splash** during hibernation (covering the NVIDIA VRAM save and the kernel-image write) and a **resume-offset guardian** that verifies at every boot that the rEFInd `resume_offset` matches the real `/swapfile` geometry, repairing it automatically if the swapfile is recreated or moved. |

### Desktop & GNOME (the macOS-like experience)

| Package | What it does |
|---|---|
| `pulsaros-theme` | **MacTahoe GTK theme** (GTK3/GTK4) and **MacTahoe icon theme**, compiled and applied system-wide and to new users via `/etc/skel`. |
| `pulsaros-gnome` | The Pulsar desktop extension layer: downloads and preinstalls a curated stack of GNOME Shell extensions, compiles and installs **Pulsar Dock** and **Liquid Glass**, and injects GSchema overrides + dconf databases that produce the macOS-like desktop out of the box. |
| `Pulsar-Dock` | Inled fork of Dash-to-Dock maintained for Pulsar OS with smart dynamic autohide, macOS bottom placement, and subtle indicators. |
| `Pulsar-Liquid-Glass` | Translucent glass effects engine for GNOME Shell, producing frosted glass visuals for dock, panels, and system popups. |
| `Pulsar-CC-27-Gnome` | Modular macOS-style **Control Center** for GNOME Shell with quick toggles for Wi-Fi, Bluetooth, AirDrop/FlyDrop, Display Brightness, and Sound. |
| `Pulsar-Notification-Center` | Native macOS-styled **Notification Center** for GNOME Shell with grouped alerts, widgets, and clean dismissal. |
| `pulsaros-global-menu` | Large in-house GNOME Shell extension (Wayland): **Mac-style global application menus** in the top bar (Apple menu, File/Edit/View…, power-off/restart dialogs with countdown) plus a full-screen **custom lock screen**. |
| `pulsaros-notchnux` | **macOS-style dynamic notch** for GNOME Shell, customized for Pulsar OS with multi-monitor support, AMOLED-black styling, media controls, system information, and interactive notifications. |
| `pulsaros-control-center` | Custom compile of **GNOME Control Center** with Pulsar OS patches: Appearance panel with Live Wallpaper engine, Liquid Glass effects, macOS-style accent colors and custom layout. |
| `pulsar-desktop-icons` | Fluid, minimalist desktop icon manager designed specifically for Pulsar OS. |
| `no-overview` | Lightweight GNOME extension that prevents the Activities Overview from launching at login, booting directly with the desktop focused. |
| `pulsaros-ui-tweaks` | Fine-grained shell configuration suite: startup behavior, icon sizes, panel margins, and fluid animations. |
| `wiggle` | Cursor shake effect: shakes and briefly enlarges the mouse pointer when moved rapidly, making it effortless to locate on large or multiple monitors. |
| `pulsaros-spotlight-launcher` | **Spotlight-like search**: native Rust/GTK4 search window with full-text file & application search powered by GNOME Tracker/TinySPARQL (`localsearch`). |
| `nautilus` | **Finder**, the Pulsar OS file manager: custom build of GNOME Files with macOS layout, real-time **folder color tagging** and per-color filters in the sidebar. |
| `pulsaros-live-wallpaper` | **Animated / video wallpaper engine and daemon**: plays MP4, WebM, GIF, MKV and WebP wallpapers natively through the Hidamari engine, synchronized across desktop, lock screen, and SDDM login screen. |
| `pulsaros-cloud` | **Cloud drives in Finder sidebar**: wrapper around `rclone` that mounts cloud accounts (Google Drive, OneDrive, iCloud) under `~/Cloud/<name>`. |

### Apps, assistants & utilities

| Package | What it does |
|---|---|
| `pulsaros-welcome` | First-boot **Welcome / OOTB** application: native onboarding with Apple-like "hello" animation, real-time configuration progress bar, live log viewer, and user setup. |
| `pulsaros-recovery` | **Recovery & installer UI** in the style of macOS Recovery: full-screen GTK4/Libadwaita interface with Recovery Utilities (Install Pulsar OS, Seafari browser, Disk Utility/GParted, Time Machine), Btrfs repair, and multi-boot installers. |
| `pulsaros-firewall` | Modern desktop firewall interface built on UFW: simple toggles, connection profiles, and port management without terminal commands. |
| `pulsaros-vpn` | Integrated VPN manager for WireGuard and OpenVPN configurations with instant top-bar status and connection control. |
| `pulsaros-feedback` | User feedback and bug reporting assistant for submitting diagnostics and improvement suggestions. |
| `pulsaros-whatsnew` | Interactive release notes and update showcase dialog displayed automatically after system upgrades. |
| `pulsaros-hblock` | System-wide ad, tracking, and malware blocker with interactive GTK4 management interface. |
| `pulsaros-timemachine` | **Continuous backup suite**: local Btrfs snapshots synced with restic to USB drives, Samba/NAS shares, and cloud storage. |
| `pulsaros-circle-to-search` | **Circle to Search**: shake mouse or press shortcut, select a screen region for real-time OCR (Tesseract), AI analysis with Sayri, or Google visual search. |
| `pulsaros-flydrop` | **FlyDrop: AirDrop-style local file sharing.** Self-contained Python daemon speaking the LocalSend v2 protocol on LAN with GTK4 UI, dynamic shelf GNOME extension, and Nautilus right-click integration. |
| `sayri` | **Siri-like AI assistant**: floating orb (GTK4 layer-shell), local speech-to-text with whisper.cpp, Piper TTS, and agentic integrations. |
| `pulsaros-update` | Graphical system update assistant for scheduled and on-demand maintenance of Arch and Debian packages. |

### Tube OS (NAS / home-server line)

| Package | What it does |
|---|---|
| `tube-os-dash` | **Tube OS Dashboard**: web dashboard and app orchestrator for Tube OS — Docker app management, storage, disk health, and cloud mounts. |
| `tubeos-installer` | **Headless web installer / OOTB setup** for Tube OS: browser-accessible installer at `http://tubeos.local` for complete disk install and setup without HDMI. |
| `tubeos-branding` | Logos, icons, and branding assets for Tube OS. |
| `tubeos-plymouth` | Animated Plymouth boot splash for Tube OS. |
| `tubeos-ui` | Specialized system utilities and management interfaces for Tube OS. |

### Support folders & scripts

| Path | What it is |
|---|---|
| `arch/` | `PKGBUILD`s packaging all Pulsar OS components for the **Arch Linux** edition of the ISO. |
| `apple-hello/` | Source of the Apple-style "hello" boot animation used by the Welcome app. |
| `.github/workflows/` | CI that compiles the packages (`build-packages.yml`) and deploys them to the Inled central repository. |
| `package-and-deploy.sh` | Builds packages into `.deb` / `.pkg.tar.zst` and deploys them to local and central repositories. |
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
| `pulsaros-notchnux` → `notchnux@adityasah.programs` | **NotchNux** by *Aditya Sah* (`Adityasah2004/NotchNux`, MIT) — an existing third-party notch/dashboard extension that Inled packages as-is and then customizes for Pulsar OS (multi-monitor notch, AMOLED-black styling, Wayland drag & drop helper, Notch section in Control Center). Upstream code is not original Inled work. |
| `hblock` | **hblock** v3.5.1 by *Héctor Molinero Fernández* (`hectorm/hblock`, MIT) — repackaged verbatim for Pulsar OS with the extra `hblock.service` / `hblock.timer` systemd units and the Pulsar blocklist defaults. |
| `pulsaros-hblock` | Original Inled GTK4/Libadwaita switch & statistics UI built **around hblock**; the blocking engine itself is upstream `hectorm/hblock`. |
| `pulsaros-flydrop` | Original Inled **Python/GTK4 + GNOME Shell** code, written as an independent reimplementation of the **LocalSend v2 wire protocol** so FlyDrop interoperates with LocalSend devices. **No LocalSend source is bundled.** The "AirDrop" side is an in-house UI/concept over the same LocalSend transport — Apple's real AWDL/BLE AirDrop protocol is *not* implemented and no Apple asset is used. |
| `pulsaros-gnome` → `pulsar-dock@inled.es` | **Dash-to-Dock** (micheleg/dash-to-dock) — Inled fork maintained at `Inled-Pulsar-OS/dash-to-dock`. |
| `pulsaros-gnome` → `liquid-glass@…` | **Liquid Glass** GNOME extension by *thinkingcoding1231* / *ryohsuke1231* — upstream repository at `ryohsuke1231/liquid-glass`. |
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
| `pulsaros-global-menu`, `pulsaros-control-center-button`, `pulsaros-recovery`, `driverman`, `pulsaros-island`, `pulsaros-update`, `dockermigrate`, `tubeos-installer`, `tubeos-plymouth`, `tubeos-branding`, `tubeos-ui`, `pulsaros-branding`, `pulsaros-bootsound`, `pulsar-pear-sound-theme`, `pulsar-boot-icons`, `pulsaros-emoji-fonts`, `pulsaros-meta` | **Original Inled / Pulsar OS work** — no third-party upstream (content may be inspired by macOS visuals, but the code is original). |
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
- `hblock` — **MIT** (© Héctor Molinero Fernández, `hectorm/hblock`), upstream
  v3.5.1 redistributed with our systemd units.
- `pulsaros-notchnux` — **MIT** (© Aditya Sah, `Adityasah2004/NotchNux`),
  Pulsar OS modifications applied on top.
- `pulsaros-flydrop` — the package code is original Inled code, **MIT-INLED**;
  it implements the **LocalSend** v2 protocol specification to interoperate with
  other LocalSend apps. LocalSend itself (© Tien Do Nam) is **Apache-2.0** and
  its source is *not* redistributed inside this package.
- `pulsaros-spotlight-launcher`, `sayri` and the rest of the in-house apps —
  original Inled code, **MIT-INLED**.

When in doubt, read the `LICENSE` / `DEBIAN/copyright` files shipped inside each
package folder.
