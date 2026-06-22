#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome Shell Extensions Asset Preparer
# ==============================================================================
# Descarga e instala localmente las extensiones GNOME Shell seleccionadas
# dentro de la estructura temporal del paquete pulsaros-gnome.
# Evita descargas durante el bootstrap y estabiliza las dependencias.
# ==============================================================================

set -e

STAGE_DIR="$1"
GNOME_VER="47" # Debian 13 Trixie por defecto usa Gnome 47

EGO_EXTENSIONS=(
    "wiggle@mechtifs"
    "search-light@icedman.github.com"
    "kiwimenu@kemma"
    "compiz-alike-magic-lamp-effect@hermes83.github.com"
    "fullscreen-to-empty-workspace2@corgijan.dev"
    "blur-my-shell@aunetx"
    "dash-to-dock@micxgx.gmail.com"
    "user-theme@gnome-shell-extensions.gcampax.github.com"
    "appmenu-is-back@fthx"
    "just-perfection-desktop@just-perfection"
    "appindicatorsupport@rgcjonas.gmail.com"
    "notification-position@drugo.dev"
)

echo "🧩 Descargando extensiones de GNOME Shell desde Extensions.gnome.org (EGO)..."

for uuid in "${EGO_EXTENSIONS[@]}"; do
    echo "Descargando: $uuid..."
    info_url="https://extensions.gnome.org/extension-info/?uuid=${uuid}&shell_version=${GNOME_VER}"
    download_path=$(curl -s "$info_url" | jq -r '.download_url')

    if [ "$download_path" != "null" ] && [ -n "$download_path" ]; then
        full_url="https://extensions.gnome.org${download_path}"
        tmp_zip="/tmp/${uuid}.zip"
        
        if curl -L -s -o "$tmp_zip" "$full_url"; then
            dest_dir="$STAGE_DIR/usr/share/gnome-shell/extensions/${uuid}"
            mkdir -p "$dest_dir"
            unzip -q -o "$tmp_zip" -d "$dest_dir"
            rm "$tmp_zip"
            echo "✅ Extensión $uuid lista."
        fi
    else
        echo "⚠️ Advertencia: No se encontró versión compatible en EGO para $uuid y GNOME $GNOME_VER."
    fi
done

# Compilar esquemas locales de las extensiones dentro de staging para que estén listos
echo "Compilando esquemas locales de extensiones..."
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -name schemas -type d 2>/dev/null | while read -r schema_path; do
    glib-compile-schemas "$schema_path" || true
done

echo "✅ Proceso de extensiones finalizado."
#!/bin/bash
