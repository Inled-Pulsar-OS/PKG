#!/bin/bash
# ==============================================================================
# Pulsar OS - Pulsar Firewall packaging preparation hook
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"

chmod 0755 "$STAGE_DIR/usr/bin/pulsar-firewall" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/bin/pulsar-firewall-prompt" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/bin/pulsar-firewall-watcher" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-firewall/firewall-helper" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-firewall/"*.py 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/applications/es.pulsaros.Firewall.desktop" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/lib/systemd/user/pulsar-firewall-watcher.service" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/polkit-1/actions/es.pulsaros.firewall.policy" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/etc/polkit-1/rules.d/60-pulsaros-firewall.rules" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/"*.svg 2>/dev/null || true

# Clean pycache
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true
