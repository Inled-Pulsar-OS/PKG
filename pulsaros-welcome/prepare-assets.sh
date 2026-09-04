#!/bin/bash
# ==============================================================================
# Pulsar OS - Welcome App Asset Preparer (Tauri RELEASE build)
# ==============================================================================
# English: Builds the React frontend and the Tauri shell in RELEASE mode with
#          the bundled dist/ assets, installs the binary at
#          /usr/lib/pulsaros-welcome/pulsaros-welcome (where the wrapper looks
#          for it) and purges all dev-mode artifacts (cargo target/, node_modules,
#          __pycache__) from the package.
# Español: Compila el frontend React y el shell Tauri en modo RELEASE con los
#          assets de dist/ empaquetados, instala el binario en
#          /usr/lib/pulsaros-welcome/pulsaros-welcome (donde lo busca el wrapper)
#          y purga todos los artefactos de desarrollo (cargo target/,
#          node_modules, __pycache__) del paquete.
#
# CRITICAL: a Tauri binary compiled in debug mode starts in "dev" behavior and
# tries to connect to the Vite dev server (devUrl, http://localhost:1420),
# showing a black window with "could not connect to localhost: connection
# refused". Shipping ANY debug binary is forbidden — this script only ever
# produces and installs release builds, and the wrapper refuses to launch the
# binary without the RELEASE_BUILD marker.
#
# Usage: prepare-assets.sh <STAGE_DIR>
#   PULSAR_WELCOME_SKIP_BUILD=1  → skip compilation (stage existing artifacts)
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "${1:?Usage: prepare-assets.sh <STAGE_DIR>}")"
PKG_ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$PKG_ROOT/usr/share/pulsaros-welcome"

echo "🎨 pulsaros-welcome: preparando assets (build RELEASE) → $STAGE_DIR"

# Directorios que NUNCA deben entrar en el paquete
JUNK_DIRS="src-tauri/target node_modules usr/bin/__pycache__"

# ------------------------------------------------------------------------------
# 0. Purga en el árbol fuente (para que futuros cp -r del empaquetador sean
#    rápidos y nunca arrastren artefactos de desarrollo)
# ------------------------------------------------------------------------------
purge_source() {
    for d in $JUNK_DIRS; do
        if [ -e "$SRC_DIR/$d" ]; then
            echo "🧹 Purgando artefactos de desarrollo del árbol fuente: $d"
            rm -rf "$SRC_DIR/$d" 2>/dev/null || true
        fi
    done
}

# ------------------------------------------------------------------------------
# 1. Frontend: dist/ ya construido tiene prioridad; si falta o hay node_modules
#    se reconstruye con vite (build de producción, nunca dev)
# ------------------------------------------------------------------------------
build_frontend() {
    if [ ! -f "$SRC_DIR/dist/index.html" ]; then
        echo "📦 dist/ ausente — compilando frontend de producción (vite)..."
        cd "$SRC_DIR"
        if [ -d node_modules ] || command -v pnpm >/dev/null 2>&1; then
            command -v pnpm >/dev/null 2>&1 && pnpm install --frozen-lockfile || npm ci
        else
            npm ci
        fi
        command -v pnpm >/dev/null 2>&1 && pnpm run build || npm run build
        [ -f "$SRC_DIR/dist/index.html" ] || { echo "❌ El build de vite no generó dist/index.html"; exit 1; }
    else
        echo "✔ dist/ ya compilado — reutilizando assets de producción"
    fi
}

# ------------------------------------------------------------------------------
# 2. Shell Tauri en RELEASE con los assets de dist/ embebidos
# ------------------------------------------------------------------------------
build_tauri() {
    cd "$SRC_DIR/src-tauri"
    echo "🦀 Compilando Tauri en RELEASE (frontendDist=../dist embebido)..."
    # CRITICAL: --features custom-protocol es OBLIGATORIO en builds de
    # producción. Sin él, el binario arranca en modo dev y espera el dev
    # server de Vite (http://localhost:1420) → "connection refused".
    cargo build --release --features custom-protocol --frozen 2>/dev/null \
        || cargo build --release --features custom-protocol
    [ -f "target/release/pulsaros-welcome" ] || { echo "❌ cargo no generó el binario release"; exit 1; }
    # Verificación: un binario de producción embebe dist/ (frontendDist), por
    # lo que pesa mucho más que uno en modo dev (~4.5 MB). Nota: devUrl aparece
    # en TODO binario (la config se embebe íntegra), así que strings/grep no
    # sirve para discriminar.
    _size=$(stat -c%s "target/release/pulsaros-welcome" 2>/dev/null || echo 0)
    _dist_size=$(du -sb "$SRC_DIR/dist" 2>/dev/null | cut -f1 || echo 0)
    if [ "${_size:-0}" -lt "$((_dist_size + 3000000))" ] 2>/dev/null; then
        echo "❌ el binario (${_size}B) no embebe dist/ (${_dist_size}B) — ¿se compiló sin --features custom-protocol?"
        exit 1
    fi
    echo "✔ Binario RELEASE con assets embebidos (${_size} bytes)"
}

# ------------------------------------------------------------------------------
# 3. Instalación en staging
# ------------------------------------------------------------------------------
install_to_stage() {
    mkdir -p "$STAGE_DIR/usr/lib/pulsaros-welcome"
    install -m 755 "$SRC_DIR/src-tauri/target/release/pulsaros-welcome" \
        "$STAGE_DIR/usr/lib/pulsaros-welcome/pulsaros-welcome"
    strip --strip-unneeded "$STAGE_DIR/usr/lib/pulsaros-welcome/pulsaros-welcome" 2>/dev/null || true
    # Marcador: el wrapper SOLO lanza el binario Tauri si existe este archivo,
    # garantizando que jamás se ejecute un binario en modo dev.
    echo "release $(date -u '+%Y-%m-%dT%H:%M:%SZ')" > "$STAGE_DIR/usr/lib/pulsaros-welcome/RELEASE_BUILD"
    echo "✔ Binario RELEASE instalado en usr/lib/pulsaros-welcome/pulsaros-welcome"
}

# ------------------------------------------------------------------------------
# 4. Purga en staging (defensa en profundidad: el empaquetador copia el árbol
#    completo ANTES de ejecutar este hook)
# ------------------------------------------------------------------------------
purge_stage() {
    for d in $JUNK_DIRS; do
        if [ -e "$STAGE_DIR/usr/share/pulsaros-welcome/$d" ]; then
            echo "🧹 Purgando artefactos de desarrollo del staging: $d"
            rm -rf "$STAGE_DIR/usr/share/pulsaros-welcome/$d"
        fi
    done
    rm -rf "$STAGE_DIR/usr/share/pulsaros-welcome/src-tauri/target" 2>/dev/null || true
}

if [ "${PULSAR_WELCOME_SKIP_BUILD:-0}" = "1" ]; then
    echo "⏭️ PULSAR_WELCOME_SKIP_BUILD=1 — sin compilación (solo staging/purga)"
else
    build_frontend
    build_tauri
    install_to_stage
fi

purge_stage
purge_source

echo "✅ pulsaros-welcome: assets preparados (RELEASE, sin artefactos dev)"
