#!/usr/bin/env bash
# Build pulsaros-control-center for Debian trixie (GNOME 48), install it in a clean
# debian:trixie container, render the Settings window under Xvfb, write a PNG.
# Local only: no deploy, no upload, no tokens. The repo is mounted read-only.
#
#   scripts/render-settings-trixie.sh [out.png]
#
# Env:
#   WORK_DIR     scratch dir for the .deb and logs (default: a new mktemp dir)
#   SKIP_BUILD=1 reuse the .deb already in WORK_DIR
#   PANEL_CLICK  "x y" sidebar click before the shot (default "100 398" = Appearance;
#                "" = the start page)
#   PROBE_CSS    file installed as ~/.config/gtk-4.0/gtk.css before launch (see
#                scripts/probe-selectors.sh)
#   VIEWS        extra views, ";"-separated "name:xdotool args"; shots go to <out>.<name>.png
set -euo pipefail

SELF="$(realpath "$0")"
REPO="$(dirname "$(dirname "$SELF")")"
IMAGE="${IMAGE:-debian:trixie}"

# ------------------------------------------------------------------ in container
if [ "${1:-}" = "--build" ]; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends fakeroot dpkg-dev patch curl ca-certificates git >/dev/null
    # Work on a copy: package-and-deploy.sh bumps Version in DEBIAN/control, and the
    # mount is read-only. chmod: dpkg-deb rejects a DEBIAN dir that is not 0755.
    mkdir /work
    cp /src/package-and-deploy.sh /work/
    cp -a /src/pulsaros-control-center /work/
    chmod -R u=rwX,go=rX /work
    cd /work
    ./package-and-deploy.sh pulsaros-control-center
    cp build/packages/pulsaros-control-center_*.deb /out/
    exit 0
fi

if [ "${1:-}" = "--render" ]; then
    export DEBIAN_FRONTEND=noninteractive
    OUT="$2"
    apt-get update -qq
    # Pre-existing gap, not changed here: the shipped package's Depends miss these 8 runtime
    # libraries (the binary does not start without them), so the harness installs them itself.
    apt-get install -y --no-install-recommends /out/pulsaros-control-center_*.deb >/tmp/install.log 2>&1 \
        || { tail -20 /tmp/install.log; exit 1; }
    apt-get install -y -qq --no-install-recommends libibus-1.0-5 libgnome-bg-4-2t64 libgnome-rr-4-2t64 \
        libgoa-backend-1.0-2 libsmbclient0 libgsound0t64 libnma-gtk4-0 libgnome-bluetooth-ui-3.0-13 >/dev/null
    apt-get install -y -qq --no-install-recommends xvfb xauth xdotool imagemagick dbus-x11 \
        adwaita-icon-theme mutter-common gnome-shell-common librsvg2-common fonts-dejavu-core libgl1-mesa-dri libglib2.0-bin >/dev/null
    gnome-control-center --version
    # The Appearance panel aborts without this schema (shipped by pulsaros-global-menu).
    cp /src/pulsaros-global-menu/usr/share/glib-2.0/schemas/org.gnome.shell.extensions.pulsaros-global-menu.gschema.xml \
        /usr/share/glib-2.0/schemas/
    glib-compile-schemas /usr/share/glib-2.0/schemas
    dpkg-query -W libgtk-4-1 libadwaita-1-0 pulsaros-control-center >/out/versions.txt
    if [ -f /probe.css ]; then
        mkdir -p ~/.config/gtk-4.0 && cp /probe.css ~/.config/gtk-4.0/gtk.css
    fi
    export XDG_CURRENT_DESKTOP=GNOME GSK_RENDERER=cairo GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 \
        NO_AT_BRIDGE=1 GTK_A11Y=none OUT PANEL_CLICK VIEWS
    dbus-run-session -- xvfb-run -a -s "-screen 0 1280x800x24" bash -c '
        gnome-control-center >/out/gcc.log 2>&1 &
        sleep 8
        shot() { import -window root -crop 980x640+0+0 +repage "$1"; }
        [ -n "$PANEL_CLICK" ] && { xdotool mousemove $PANEL_CLICK click 1; sleep 4; }
        shot "$OUT"
        IFS=";"
        for v in $VIEWS; do
            IFS=" "; xdotool ${v#*:}; sleep 3; shot "${OUT%.png}.${v%%:*}.png"; IFS=";"
        done
        kill %1 || true'
    exit 0
fi

# ------------------------------------------------------------------ on the host
OUT_PNG="$(realpath -m "${1:-render-settings-trixie.png}")"
WORK_DIR="$(realpath -m "${WORK_DIR:-$(mktemp -d)}")"
mkdir -p "$WORK_DIR" "$(dirname "$OUT_PNG")"
RO="-v $REPO:/src:ro -v $WORK_DIR:/out"

if [ "${SKIP_BUILD:-}" != 1 ]; then
    rm -f "$WORK_DIR"/*.deb
    docker run --rm $RO "$IMAGE" bash /src/scripts/render-settings-trixie.sh --build 2>&1 | tee "$WORK_DIR/build.log" | tail -5
    [ "${PIPESTATUS[0]}" = 0 ] || { echo "build failed, see $WORK_DIR/build.log" >&2; exit 1; }
fi

PROBE=""
[ -n "${PROBE_CSS:-}" ] && PROBE="-v $(realpath "$PROBE_CSS"):/probe.css:ro"
docker run --rm $RO $PROBE -e PANEL_CLICK="${PANEL_CLICK-100 398}" -e VIEWS="${VIEWS:-}" "$IMAGE" \
    bash /src/scripts/render-settings-trixie.sh --render "/out/$(basename "$OUT_PNG")"
[ "$WORK_DIR" = "$(dirname "$OUT_PNG")" ] || mv "$WORK_DIR"/*.png "$(dirname "$OUT_PNG")/"
echo "PNG: $OUT_PNG  (deb, logs: $WORK_DIR)"
