#!/bin/bash
# ==============================================================================
# Pulsar OS - Pulsar VPN & Phone packaging preparation hook
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"

chmod 0755 "$STAGE_DIR/usr/bin/pulsar-vpn" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/bin/pulsar-bridge" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-vpn/vpn_gui.py" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-vpn/bridge_daemon.py" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/applications/es.pulsaros.Vpn.desktop" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/lib/systemd/user/pulsar-bridge.service" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/"*.svg 2>/dev/null || true

# Clean pycache
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true
