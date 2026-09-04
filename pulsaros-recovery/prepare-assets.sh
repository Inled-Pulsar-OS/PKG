#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar OS Recovery Assistant assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

# Build native Rust binary
mkdir -p "$STAGE_DIR/usr/bin"
if [ -d "$SRC_DIR/rust-recovery" ]; then
    echo "🦀 Compiling Pulsar OS Recovery Assistant (Rust)..."
    (
        cd "$SRC_DIR/rust-recovery"
        cargo build --release
    )
    install -m 755 "$SRC_DIR/rust-recovery/target/release/pulsar-recovery-assistant" "$STAGE_DIR/usr/bin/pulsar-recovery-assistant"
    # Also update local usr/bin
    mkdir -p "$SRC_DIR/usr/bin"
    cp -f "$SRC_DIR/rust-recovery/target/release/pulsar-recovery-assistant" "$SRC_DIR/usr/bin/pulsar-recovery-assistant"
fi

# Ensure helper binaries and scripts are executable
if [ -f "$STAGE_DIR/usr/bin/pulsaros-recovery" ]; then
    chmod 755 "$STAGE_DIR/usr/bin/pulsaros-recovery"
fi
if [ -f "$STAGE_DIR/usr/bin/pulsaros-recovery-desktop-setup" ]; then
    chmod 755 "$STAGE_DIR/usr/bin/pulsaros-recovery-desktop-setup"
fi

# Clean up build artifacts and source code from staging directory so they are not included in the deb
rm -rf "$STAGE_DIR/rust-recovery" "$STAGE_DIR/.git" "$STAGE_DIR/.gitignore"
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar OS Recovery staging preparation complete."
