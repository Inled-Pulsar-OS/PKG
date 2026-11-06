#!/bin/bash
# Builds pulsaros-airpods exactly as CI does: same container image, same install-build-deps.sh,
# same package-and-deploy.sh. Packages land in build/packages/ (.deb) and arch/build/packages/ (.pkg.tar.zst).
# Usage: pulsaros-airpods/build-local.sh [debian|arch|both] [branch]   (default: both stable)
set -e
distro="${1:-both}"; branch="${2:-stable}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
uid="$(id -u)"; gid="$(id -g)"

if [ "$distro" = debian ] || [ "$distro" = both ]; then
    docker run --rm -v "$repo:/workspace" -w /workspace -e CI=true -e USER=root -e HOME=/root \
        -e DEBIAN_FRONTEND=noninteractive "$("$repo/ci-image.sh" debian "$branch")" bash -ec "
        apt-get update
        apt-get install -y --no-install-recommends dpkg-dev fakeroot jq curl git rsync sudo ca-certificates
        ./install-build-deps.sh debian pulsaros-airpods
        ./package-and-deploy.sh pulsaros-airpods --branch '$branch'
        chown -R $uid:$gid /workspace/build"
fi

if [ "$distro" = arch ] || [ "$distro" = both ]; then
    docker run --rm -v "$repo:/workspace" -w /workspace -e CI=true "$("$repo/ci-image.sh" arch "$branch")" bash -ec "
        pacman-key --init; pacman-key --populate archlinux
        pacman -Syu --noconfirm base-devel git sudo jq curl
        ./install-build-deps.sh arch pulsaros-airpods
        useradd -m builder; echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers
        chown -R builder:builder /workspace
        sudo -u builder bash -c 'cd /workspace/arch && ./package-and-deploy.sh pulsaros-airpods --branch $branch'
        chown -R $uid:$gid /workspace/arch/build /workspace/arch/pkgbuilds/pulsaros-airpods"
fi
