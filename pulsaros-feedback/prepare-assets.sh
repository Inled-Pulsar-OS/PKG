#!/bin/bash
# ==============================================================================
# Pulsar OS - Feedback app packaging preparation hook
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"

# Permissions
chmod 0755 "$STAGE_DIR/usr/bin/pulsaros-feedback" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-feedback/feedback.py" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/applications/es.inled.PulsarFeedback.desktop" 2>/dev/null || true
chmod 0644 "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/"*.svg 2>/dev/null || true

# Clean pycache
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true
