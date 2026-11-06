#!/bin/bash
# Builds pulsaros-airpods exactly as CI does: same container image, same install-build-deps.sh,
# same package-and-deploy.sh. Packages land in build/packages/ (.deb) and arch/build/packages/ (.pkg.tar.zst).
# Usage: pulsaros-airpods/build-local.sh [debian|arch|both] [branch]   (default: both stable)
set -euo pipefail
distro="${1:-both}"; branch="${2:-stable}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "$distro" in debian|arch|both) ;; *) echo "unknown distro: $distro" >&2; exit 1 ;; esac
case "$branch" in stable|unstable|forky|rolling) ;; *) echo "unknown branch: $branch" >&2; exit 1 ;; esac
uid="$(id -u)"; gid="$(id -g)"

# package-and-deploy.sh bumps Version: in the committed control file on every build; put it back.
control="$repo/pulsaros-airpods/DEBIAN/control"
saved="$(mktemp)"; cp "$control" "$saved"
trap 'cp "$saved" "$control"; rm -f "$saved"' EXIT

if [ "$distro" = debian ] || [ "$distro" = both ]; then
    docker run --rm -v "$repo:/workspace" -w /workspace -e CI=true -e USER=root -e HOME=/root \
        -e BRANCH="$branch" -e DEBIAN_FRONTEND=noninteractive "$("$repo/ci-image.sh" debian)" bash -ec "
        trap 'chown -R $uid:$gid /workspace/build' EXIT
        apt-get update
        apt-get install -y --no-install-recommends dpkg-dev fakeroot jq curl git rsync sudo ca-certificates
        ./install-build-deps.sh debian pulsaros-airpods
        ./package-and-deploy.sh pulsaros-airpods --branch \"\$BRANCH\""
fi

if [ "$distro" = arch ] || [ "$distro" = both ]; then
    docker run --rm -v "$repo:/workspace" -w /workspace -e CI=true -e BRANCH="$branch" "$("$repo/ci-image.sh" arch)" bash -ec "
        trap 'chown -R $uid:$gid /workspace/arch/build /workspace/arch/pkgbuilds/pulsaros-airpods' EXIT
        pacman-key --init; pacman-key --populate archlinux
        pacman -Syu --noconfirm base-devel git sudo jq curl
        ./install-build-deps.sh arch pulsaros-airpods
        useradd -m builder; echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers
        chown -R builder:builder /workspace
        sudo -u builder env BRANCH=\"\$BRANCH\" bash -c 'cd /workspace/arch && ./package-and-deploy.sh pulsaros-airpods --branch \"\$BRANCH\"'"
fi
