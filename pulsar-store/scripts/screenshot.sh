#!/usr/bin/env bash
# Open a page of the real app on the current Wayland session and save a screenshot.
#   scripts/screenshot.sh <page> <out.png> [light|dark]
# <page> is a --page value: discover, apps, extensions, sayri, installed, updates,
# search:<query>, item:<id>. Uses throwaway cache and config directories.
# Needs hyprctl and grim. A window appears on screen for a few seconds.
set -euo pipefail
cd "$(dirname "$0")/.."
page=${1:?page}; out=${2:?output file}; scheme=${3:-dark}
bin=${PULSAR_STORE_BIN:-target/debug/pulsar-store}
tmp=$(mktemp -d)
pid=""
cleanup() {
    [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    # $tmp is the directory mktemp just created; nothing else is touched.
    rm -r -- "$tmp"
}
trap cleanup EXIT

# SHOT_CACHE_SEED: a folder copied into the cache first (for example a test catalog).
# Pair it with PULSAR_STORE_OFFLINE=1 so the app keeps that catalog.
if [ -n "${SHOT_CACHE_SEED:-}" ]; then
    mkdir -p "$tmp/cache"
    cp -r "$SHOT_CACHE_SEED"/. "$tmp/cache/"
fi

ADW_DEBUG_COLOR_SCHEME="prefer-$scheme" XDG_CACHE_HOME="$tmp/cache" XDG_CONFIG_HOME="$tmp/config" \
    LANG=${LANG_FOR_SHOT:-en_US.UTF-8} LC_ALL=${LANG_FOR_SHOT:-en_US.UTF-8} "$bin" ${SHOT_ARGS:---page "$page"} >"$tmp/log" 2>&1 &
pid=$!

find_window() {
    hyprctl -j clients | python3 -c '
import json, sys
pid = int(sys.argv[1])
for c in json.load(sys.stdin):
    if c.get("class") == "es.inled.PulsarStore" and c["pid"] == pid:
        x, y = c["at"]
        w, h = c["size"]
        print(f"{x},{y} {w}x{h}")
        break
' "$pid"
}

geometry=""
for _ in $(seq 1 40); do
    sleep 0.5
    geometry=$(find_window)
    [ -n "$geometry" ] && break
done
[ -n "$geometry" ] || { echo "window did not appear" >&2; cat "$tmp/log" >&2; exit 1; }
# Only capture when our window is the focused one, so nothing else on screen is included.
active_pid=$(hyprctl -j activewindow | python3 -c 'import json,sys; print(json.load(sys.stdin).get("pid", 0))')
if [ "$active_pid" != "$pid" ]; then
    echo "window is not in front (active pid $active_pid); refusing to capture the screen" >&2
    exit 1
fi
sleep "${SHOT_DELAY:-5}"   # let the catalog, states, and icons load

# Check again right before capturing: another window may have taken the spot meanwhile.
geometry=$(find_window)
active_pid=$(hyprctl -j activewindow | python3 -c 'import json,sys; print(json.load(sys.stdin).get("pid", 0))')
if [ -z "$geometry" ] || [ "$active_pid" != "$pid" ]; then
    echo "window changed before capture (active pid $active_pid); refusing to capture the screen" >&2
    exit 1
fi
grim -g "$geometry" "$out"
echo "saved $out"
