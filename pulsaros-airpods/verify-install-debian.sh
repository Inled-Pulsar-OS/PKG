#!/bin/bash
# Installs the pulsaros-airpods .deb in a clean debian:trixie container and checks each acceptance criterion.
# Usage: pulsaros-airpods/verify-install-debian.sh [path/to/pulsaros-airpods_*.deb]   (default: newest in build/packages)
# Prints a PASS/FAIL table; exit 1 if any check fails.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
deb="${1:-$(ls -t "$repo"/build/packages/pulsaros-airpods_*.deb 2>/dev/null | head -1 || true)}"
[ -f "$deb" ] || { echo "no .deb found; run pulsaros-airpods/build-local.sh debian stable" >&2; exit 2; }

docker run --rm -v "$(realpath "$deb"):/pkg.deb:ro" -e DEBIAN_FRONTEND=noninteractive debian:trixie bash -c '
rc=0
row() { if [ "$1" = 0 ]; then s=PASS; else s=FAIL; rc=1; fi; printf "| %s | %s |\n" "$2" "$s"; }
apt-get update -qq >/dev/null 2>&1
i=0; apt-get install -y --no-install-recommends /pkg.deb >/tmp/install.log 2>&1 || i=1
[ $i != 0 ] && tail -20 /tmp/install.log >&2
echo "| Check | Result |"; echo "|---|---|"
row $i "deb installs, dependencies resolve"
ext=/usr/share/gnome-shell/extensions/airpods@pulsaros.org
c=0; [ -f $ext/metadata.json ] && [ -f $ext/extension.js ] || c=1; row $c "extension files under $ext"
c=0; tr -d "\n " < $ext/metadata.json | grep -Eq "\"shell-version\":\[[^]]*\"48\"" 2>/dev/null || c=1; row $c "metadata shell-version lists 48"
c=0; command -v librepods >/dev/null || c=1; row $c "librepods binary installed"
c=0; command -v librepods-ctl >/dev/null || c=1; row $c "librepods-ctl binary installed"
c=0; ls /usr/lib/systemd/user/*librepods*.service >/dev/null 2>&1 || c=1; row $c "systemd user unit installed"
exit $rc'
