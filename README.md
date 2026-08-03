# Pulsar OS Packages (PKG)

This repository contains the **source packages** that make up Pulsar OS. PulsarOS is fully declarative: every package is defined here, built in CI and deployed to the Inled APT repository (and the Arch repository), and then pulled into the ISO at build time.

## Repository structure

| Path | Description |
|------|-------------|
| `<package>/` | Debian package sources. Each folder is a package with a `DEBIAN/control` file, the payload files (`usr/`, `etc/`, …) and usually a `prepare-assets.sh` script |
| `arch/pkgbuilds/` | Arch Linux `PKGBUILD`s (same packages, Arch edition) |
| `build/` | Local build output: `packages/` (.deb files) and `pkg-staging/` (staging dirs) |
| `package-and-deploy.sh` | Builds one or all packages into `.deb` files and optionally deploys them to the Inled APT repo |
| `arch/package-and-deploy.sh` | Same for Arch: builds `.pkg.tar.zst` archives with `makepkg` and deploys to the Inled Arch repo |
| `test-*.sh`, `test-*.py` | Testing helpers to validate packages in a chroot, QEMU/installer scenarios, bootsound, etc. |
| `.github/workflows/` | CI that builds and deploys packages on tag/release |

## Package list

### System identity & branding
| Package | What it does |
|---------|--------------|
| `pulsaros-branding` | System identity files: `/etc/os-release`, `/etc/issue`, hostnames. Rebrands Debian/Arch as **Pulsar OS**. Replaces `base-files` |
| `pulsaros-theme` | MacTahoe **GTK theme** and icon theme, compiled and packaged for Pulsar OS |
| `pulsaros-plymouth` | macOS-like **Plymouth boot splash**; replaces the default Debian logos |
| `pulsaros-bootsound` | Plays the official **startup sound** at boot via a systemd service (`aplay`) |

### Bootloaders & login
| Package | What it does |
|---------|--------------|
| `pulsaros-grub` | macOS-like **GRUB** bootloader theme and configuration |
| `pulsaros-refind` | **rEFInd** bootloader configuration with the macOS theme |
| `pulsaros-sddm` | **SDDM** as display manager with the Apple Tahoe theme (keeps gdm3 libs so GNOME screen locking still works) |

### Desktop / GNOME
| Package | What it does |
|---------|--------------|
| `pulsaros-gnome` | Selected GNOME Shell extensions plus GSchema overrides and dconf databases to get a **macOS-like desktop**; replaces/conflicts with Dash to Dock |
| `pulsaros-global-menu` | GNOME Shell extension: **macOS-style global menu bar** in the top bar (GNOME 45-50, Wayland) |
| `pulsaros-spotlight-launcher` | GNOME Shell extension: **Spotlight-like search launcher** icon in the top bar (GNOME 45-50, Wayland) |
| `pulsaros-control-center-button` | GNOME Shell extension: **Control Center quick-settings button** that opens `gnome-control-center` |
| `pulsaros-effects-settings` | Desktop **effects switcher**: toggles between *Blur my Shell* and *Liquid Glass* (acrylic glassmorphism) and auto-tunes Dash to Dock for optimal rendering |
| `gnome-macos-remap-wayland` | **macOS keyboard remap** for GNOME on Wayland (based on `xremap`) |
| `spotlight-gtk` *(Arch)* | Apple-like **Spotlight launcher** app built with GTK4 + Libadwaita |

### System utilities & installer
| Package | What it does |
|---------|--------------|
| `pulsaros-essential` | Essential config and utilities: Fildem HUD, network interface config, global locales, passwordless sudo and polkit. Pulls in the core app set (droidtux, macboat, appinstall, seafari, etc.) |
| `pulsaros-calamares` | **Calamares installer** branding, slideshows, settings, launchers and autostart for the live installer; replaces `calamares-settings-debian` |
| `pulsaros-recovery` | Apple-like **Recovery Utilities + Disk selector** UI shown before Calamares launches in the installer |
| `pulsaros-welcome` | **Welcome/onboarding** app: Apple-like "hello" animation and guides for resolution, Bluetooth, GSConnect, USB debugging, macOS, and feedback |

## Building a package

```bash
# Build a single package locally
./package-and-deploy.sh pulsaros-theme

# Build ALL packages
./package-and-deploy.sh all

# Build and deploy everything to the Inled APT repo
./package-and-deploy.sh all --deploy
```

Arch equivalents live in `arch/`:

```bash
cd arch
./package-and-deploy.sh pulsaros-theme            # single package
./package-and-deploy.sh all --deploy --branch stable
```

The ISO build script (`/ISO`) can consume the packages from this repo's `build/packages/` folder with the `--local` flag.
