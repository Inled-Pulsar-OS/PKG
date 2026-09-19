#!/usr/bin/env bash
# ==============================================================================
# Tube OS TV UI (PR 158 tvOS Launcher) - Asset Preparation Script
# Builds TypeScript/Vite frontend and syncs to usr/share/tubeos-ui/
# ==============================================================================

set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_SHARE="${BASE_DIR}/usr/share/tubeos-ui"
TARGET_BIN="${BASE_DIR}/usr/bin"

mkdir -p "${TARGET_SHARE}" "${TARGET_BIN}"

cd "${BASE_DIR}"

if [[ -f "${BASE_DIR}/package.json" ]]; then
    if [[ ! -d "${BASE_DIR}/node_modules" ]]; then
        echo ">>> Installing tubeos-ui npm dependencies..."
        npm install --prefer-offline 2>/dev/null || npm install
    fi
    echo ">>> Building tubeos-ui tvOS frontend bundle with Vite..."
    npx vite build
    
    echo ">>> Syncing built assets to ${TARGET_SHARE}..."
    rm -rf "${TARGET_SHARE:?}"/*
    cp -rf dist/* "${TARGET_SHARE}/"
fi

if [[ -f "${BASE_DIR}/src-tauri/target/release/appletv-launcher" ]]; then
    cp -f "${BASE_DIR}/src-tauri/target/release/appletv-launcher" "${TARGET_BIN}/tubeos-tv"
    chmod 755 "${TARGET_BIN}/tubeos-tv"
fi

echo "✅ Tube OS TV UI assets prepared successfully."
