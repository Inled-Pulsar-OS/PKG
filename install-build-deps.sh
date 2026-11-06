#!/bin/bash
# Installs the build dependencies a package declares in <package>/build-deps.txt.
# Usage (as root, inside the build container): install-build-deps.sh <debian|arch> <pkg[,pkg...]|all>
# Packages without a build-deps.txt are skipped, so other packages build exactly as before.
set -euo pipefail
distro="${1:-}"; list="${2:-}"
case "$distro" in debian|arch) ;; *) echo "usage: $0 <debian|arch> <pkg[,pkg...]|all>" >&2; exit 1 ;; esac
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ "$list" = "all" ]; then
    files=("$root"/*/build-deps.txt)
else
    files=()
    IFS=',' read -ra names <<< "$list"
    for n in "${names[@]}"; do files+=("$root/${n//[[:space:]]/}/build-deps.txt"); done
fi
pkgs=()
for f in "${files[@]}"; do
    [ -f "$f" ] || continue
    read -ra line <<< "$(sed -n "s/^$distro: *//p" "$f")"
    pkgs+=("${line[@]}")
done
[ ${#pkgs[@]} -gt 0 ] || exit 0
echo "Installing build dependencies ($distro): ${pkgs[*]}"
case "$distro" in
    debian) DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${pkgs[@]}" ;;
    arch) pacman -S --needed --noconfirm "${pkgs[@]}" ;;
    *) echo "unknown distro: $distro" >&2; exit 1 ;;
esac
