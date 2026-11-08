#!/bin/bash
# Build the Pulsar OS Arch ISO from local PKG packages inside a privileged Arch container.
# Usage: scripts/build-iso-docker.sh [extra build-iso.sh flags]   (default: --arch --local --grub --minimal)
# Layout: PKG and ISO must be sibling dirs; both are mounted because build-iso.sh uses ../PKG.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="pulsaros-iso-builder"
FLAGS=("$@"); [ ${#FLAGS[@]} -eq 0 ] && FLAGS=(--arch --local --grub --minimal)

docker build -t "$IMAGE" - <<DOCKERFILE
FROM archlinux:latest
RUN pacman -Syu --noconfirm --needed base-devel git sudo arch-install-scripts \
rsync jq curl unzip wget squashfs-tools libisoburn sassc patchelf grub mtools \
dosfstools fakeroot meson blueprint-compiler pacman-contrib imagemagick psmisc \
python-build python-installer python-setuptools python-wheel \
 && useradd -m -u $(id -u) builder && echo 'builder ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/builder
USER builder
RUN git clone --quiet https://aur.archlinux.org/yay-bin.git /tmp/yay-bin && cd /tmp/yay-bin && makepkg -si --noconfirm
USER root
DOCKERFILE

exec docker run --rm --privileged --name pulsaros-iso-build \
  -v "$ROOT":/work -w /work/ISO -u builder "$IMAGE" \
  bash -c "./build-iso.sh ${FLAGS[*]}"
