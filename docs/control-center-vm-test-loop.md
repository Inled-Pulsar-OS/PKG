# Test loop: Settings app in the QEMU VM

Answers #49. Host: Omarchy. Guest: Pulsar OS Arch ISO.

## Why

The redesign must ship in all of Pulsar OS. Change it in `pulsaros-control-center` (Arch: `arch/pkgbuilds/gnome-control-center`), not in a running guest. Never `pacman -U` into the VM: the ISO must carry the change.

## Loop

1. Edit the overlay or patch in `arch/pkgbuilds/gnome-control-center/`.
2. Bump `pkgrel` in its `PKGBUILD`.
3. Rebuild only this package (Arch container, about 4 min 15 s). Output: `arch/build/packages/gnome-control-center-<ver>-x86_64.pkg.tar.zst`.
4. Rebuild the ISO from local packages: `scripts/build-iso-docker.sh` (default flags `--arch --local --grub --minimal`). First build took 35 min 52 s; reuse the cached base, no `--clean-base`.
5. Boot it: `ISO/run-qemu.sh` (normal user, not `sudo`). It picks the newest `ISO/build/pulsaros-*.iso`.
6. In the guest, open Settings. Take the screenshot yourself.

Name shots `before-<panel>-<light|dark>.png` and `after-<panel>-<light|dark>.png` in `~/Pictures/control-center/vm/`. Compare with the reference set in `~/Pictures/control-center/`.

## To confirm on first change

- Exact single-package build command (`arch/package-and-deploy.sh gnome-control-center` or the Docker build used for `1:51.1-8`).
- ISO rebuild time with a warm cache.
- That `--local` picks up the new package and not a cached one.
