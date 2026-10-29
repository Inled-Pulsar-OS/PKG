#!/bin/bash
# Stages the package contents under $1 (the root of the package tree).
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Preparing Pulsar Store for staging in $STAGE_DIR..."
cd "$SRC_DIR"

# Build when a binary is missing or any source is newer than the main binary.
BIN=target/release/pulsar-store
if [ ! -f "$BIN" ] || [ -n "$(find src Cargo.toml Cargo.lock assets -newer "$BIN" -type f -print -quit)" ]; then
    echo "Building (release)..."
    cargo build --release --locked
fi

install -Dm755 "$BIN" "$STAGE_DIR/usr/bin/pulsar-store"
install -Dm755 target/release/pulsar-storectl "$STAGE_DIR/usr/bin/pulsar-storectl"
install -Dm755 target/release/pulsar-store-updated "$STAGE_DIR/usr/bin/pulsar-store-updated"

install -Dm644 assets/catalog/catalog.json "$STAGE_DIR/usr/share/pulsar-store/catalog/catalog.json"

for desktop in assets/desktop/*.desktop; do
    install -Dm644 "$desktop" "$STAGE_DIR/usr/share/applications/$(basename "$desktop")"
done
install -Dm644 assets/mime/pulsar-store.xml "$STAGE_DIR/usr/share/mime/packages/pulsar-store.xml"
for icon in assets/icons/*.svg; do
    install -Dm644 "$icon" "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/$(basename "$icon")"
done
install -Dm644 assets/systemd/pulsar-store-updated.service \
    "$STAGE_DIR/usr/lib/systemd/user/pulsar-store-updated.service"

# Translations: compile every .po into a .mo under the gettext tree.
for po in assets/locale/*.po; do
    [ -e "$po" ] || continue
    lang="$(basename "$po" .po)"
    install -d "$STAGE_DIR/usr/share/locale/$lang/LC_MESSAGES"
    msgfmt --check -o "$STAGE_DIR/usr/share/locale/$lang/LC_MESSAGES/pulsar-store.mo" "$po"
done

echo "Pulsar Store staging complete."
