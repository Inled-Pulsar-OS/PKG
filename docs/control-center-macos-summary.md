# Session summary: Control Center macOS look

Date: 2026-11-06 to 2026-11-08. Repo: `Victor1890/pulsaros-PKG`.

## Origin

- Follow-up: `/setup-matt-pocock-skills`, then `/wayfinder` with a loose idea.
- The idea: make `pulsaros-control-center` look and feel like macOS System Settings.
- Scope named by the owner: layout, behavior, color scheme and feel.
- Reference set: six macOS 26/27 System Settings screenshots in `~/Pictures/control-center/`.

## Why it needs a build and a VM

- `pulsaros-control-center` is a patched `gnome-control-center` 51.1 (Arch only for this effort).
- The dev machine runs Omarchy, which cannot run the patched Settings app.
- Testing therefore uses a QEMU VM booted from the Pulsar OS Arch ISO.
- The ISO is built from the local PKG packages, so changes can be seen end to end.

## Wayfinder map

- Principal branch: `feat/control-center-macos`, from `main`, pushed to origin.
- Map issue: "Control Center: macOS 27 System Settings look and feel" (#48).
- Destination: a design spec plus a board of build issues.
- Decisions taken at charting:
  - Scope is the Settings app only; the Quick Settings extension `Pulsar-CC-27-Gnome` is out of scope.
  - Fidelity: macOS layout and feel on native libadwaita widgets.
  - Shell first, then Appearance, General/About, Wi-Fi, Bluetooth, Displays, Sound.
  - Blur is deferred; GTK4 cannot blur.
  - Arch only; Debian is not yet specified.
  - New styling goes into CSS and resources, not whole-file overlays.
- Glossary: `GLOSSARY.md` (Settings app, Quick Settings, Reference set, Principal branch, Test loop).

Tickets:

- Test loop: build and run the Settings app in a QEMU VM (#49, claimed, open).
- Sidebar and styling hooks research (#50, closed).
- Accent color and theme support research (#51, closed).
- Sidebar grouping, order and icon colors (#52, grilling, open).
- Shell look prototype (#53, blocked by #49).
- One color palette for the Settings app (#54, grilling, open).

Research findings:

- Sidebar is a `GtkListBox`; style it with a `pulsar.css` appended to `shell/style.css` in `prepare()`.
- Arch overlay path is `arch/pkgbuilds/gnome-control-center/overlay`.
- GNOME 51 has `accent-color` with nine values and no multicolor; `color-scheme` has no time-based Auto.
- Three palettes conflict: libadwaita, older GNOME hex values in the overlay, MacTahoe.
- Findings branches: `research/cc-sidebar-hooks`, `research/cc-accent-theme` (no PRs).

## Build and test loop

- Package build: `gnome-control-center 1:51.1-8` built in an `archlinux` Docker container in about 4 min 15 s.
- ISO build: `ISO/build-iso.sh --arch --local --grub --minimal` inside a privileged Arch container.
- Wrapper: `PKG/scripts/build-iso-docker.sh` (uncommitted).
- Result: `ISO/build/pulsaros-stable-arch-grub.iso`, 4.2 GB, built in 35 min 52 s.
- The ISO has not been booted yet.

Problems fixed on the way:

- Missing `imagemagick` and `psmisc` in the build image.
- `clean_orphan_packages()` in `arch/package-and-deploy.sh` deleted `autokey-common`; it is now on the allow-list.
- `autokey` needed `yay-bin` and Python build tools in the image.
- A stale cached base system had no kernel presets; `--clean-base` rebuilt it.
- `ISO/run-qemu.sh` could not find the UEFI firmware.

`run-qemu.sh` rewrite (ISO repo, branch `fix/run-qemu-ovmf-and-iso-detection`, uncommitted):

- ISO resolves from the argument, `$ISO`, or the newest `build/pulsaros-*.iso`.
- Finds split OVMF CODE and VARS files and uses them as pflash drives.
- Keeps the disk and UEFI variables in `~/.local/share/pulsar-vm`.
- Overrides: `RAM`, `CPUS`, `DISK_SIZE`, `VM_DIR`, `FRESH`, `NO_KVM`.
- Run it as a normal user, not with `sudo`.

## Open items

- Boot the ISO and confirm the patched Settings app launches (closes #49).
- Choose the color palette (#54) and sidebar grouping (#52).
- Wire the push guard for `main` (`./install.sh --no-hooks --protect-main <pack-dir>`).
- Commit the uncommitted work in PKG and ISO.
- Unverified research claims: profile row position and start-typing capture in `cc-window.c`.
