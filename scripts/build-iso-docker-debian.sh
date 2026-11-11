#!/bin/bash
# Build the Pulsar OS Debian ISO from local PKG .deb packages inside a privileged Debian container.
# Usage: scripts/build-iso-docker-debian.sh [extra build-iso.sh flags]   (default: --debian --local --grub --minimal)
# Layout: PKG and ISO must be sibling dirs; both are mounted because build-iso.sh uses ../PKG.
# Build the local .deb packages first (./package-and-deploy.sh) so --local finds them in ISO/build/packages.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="pulsaros-iso-builder-debian"
FLAGS=("$@"); [ ${#FLAGS[@]} -eq 0 ] && FLAGS=(--debian --local --grub --minimal)

docker build -t "$IMAGE" - <<DOCKERFILE
FROM debian:trixie
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
sudo git ca-certificates gnupg debootstrap mmdebstrap debian-archive-keyring \
fakeroot dpkg-dev rsync jq curl unzip wget xz-utils zstd squashfs-tools xorriso mtools dosfstools \
grub-common grub-pc-bin grub-efi-amd64-bin sbsigntool imagemagick psmisc sassc python3 \
 && apt-get clean \
 && useradd -m -u $(id -u) builder && echo 'builder ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/builder
DOCKERFILE

exec docker run --rm --privileged --name pulsaros-iso-build-debian \
  -v "$ROOT":/work -w /work/ISO -u builder "$IMAGE" \
  bash -c "./build-iso.sh ${FLAGS[*]}"
