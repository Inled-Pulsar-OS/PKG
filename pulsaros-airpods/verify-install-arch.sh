#!/bin/bash
# Installs the built pulsaros-airpods package in a clean archlinux container and prints a PASS/FAIL table.
# Usage: pulsaros-airpods/verify-install-arch.sh [path/to/pulsaros-airpods-*.pkg.tar.zst]
# Default: newest arch/build/packages/pulsaros-airpods-*.pkg.tar.zst (see build-local.sh arch). Exit 1 on any FAIL.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pkg="${1:-$(ls -t "$repo"/arch/build/packages/pulsaros-airpods-*.pkg.tar.zst 2>/dev/null | head -1 || true)}"
[ -f "$pkg" ] || { echo "no package found; run pulsaros-airpods/build-local.sh arch" >&2; exit 2; }
pkg="$(realpath "$pkg")"

docker run --rm -i -v "$pkg:/pkg/p.pkg.tar.zst:ro" "$("$repo/ci-image.sh" arch)" bash -s <<'INNER'
ext=/usr/share/gnome-shell/extensions/airpods@pulsaros.org
rc=0
echo "| Check | Result |"; echo "|---|---|"
row() { # name, command...
    local n="$1"; shift
    if "$@" >/dev/null 2>&1; then echo "| $n | PASS |"; else echo "| $n | FAIL |"; rc=1; fi
}
pacman -Syu --noconfirm >/tmp/sync.log 2>&1 || { cat /tmp/sync.log; exit 3; }
row "pacman -U installs, deps resolve" pacman -U --noconfirm /pkg/p.pkg.tar.zst
row "extension.js and metadata.json under $ext" test -f $ext/extension.js -a -f $ext/metadata.json
row "metadata shell-version lists 51" grep -q '"51"' $ext/metadata.json
row "metadata uuid matches directory" grep -q '"uuid": "airpods@pulsaros.org"' $ext/metadata.json
row "/usr/bin/librepods executable" test -x /usr/bin/librepods
row "/usr/bin/librepods-ctl executable" test -x /usr/bin/librepods-ctl
row "librepods shared libs all resolve" bash -c '! ldd /usr/bin/librepods | grep -q "not found"'
row "systemd user unit librepods.service" test -f /usr/lib/systemd/user/librepods.service
row "pacman -Qk file integrity" pacman -Qk pulsaros-airpods
exit $rc
INNER
