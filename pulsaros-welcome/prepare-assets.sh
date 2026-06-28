#!/bin/bash
# ==============================================================================
# Pulsar OS - Welcome Application Asset Preparer
# ==============================================================================
# English: Downloads necessary logos and assets for the welcome slide-deck
#          during package compilation.
# Español: Descarga los logos y recursos necesarios para la presentación de bienvenida
#          durante la compilación del paquete.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
UI_DIR="$STAGE_DIR/usr/share/pulsaros-welcome/ui"

echo "📥 Descargando recursos y logos de bienvenida..."
mkdir -p "$UI_DIR"

wget -q -O "$UI_DIR/macboat.png" "https://hosted.inled.es/macboat.png"
wget -q -O "$UI_DIR/droidtux.png" "https://hosted.inled.es/droidtux.png"
wget -q -O "$UI_DIR/winboat.svg" "https://hosted.inled.es/winboat_logo.NqN8dmd9.svg"

echo "✅ Recursos de bienvenida descargados correctamente en staging."
