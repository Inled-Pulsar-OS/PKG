#!/bin/bash
# ==============================================================================
# Pulsar OS - rEFInd Theme Asset Preparer
# ==============================================================================
# English: Downloads the macOS rEFInd theme and places it in the package staging.
# Español: Descarga e instala en el paquete el tema rEFInd de macOS.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-refind-build"
THEME_REPO="https://github.com/Inled-Pulsar-OS/refind-mac-theme"

echo "🎨 Descargando tema de rEFInd desde GitHub (depth=1)..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clonar con depth=1, HTTP/1.1 y límites de velocidad
git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"

# Copiar el tema a la ruta adecuada de staging para rEFInd
# Debe ser 'rEFInd-Regular-Dark' para coincidir con las rutas internas de theme.conf
DEST_DIR="$STAGE_DIR/usr/share/refind/themes/rEFInd-Regular-Dark"
mkdir -p "$DEST_DIR"
cp -r "$TEMP_BUILD/theme"/* "$DEST_DIR/"

# English: Strip out hardcoded test menuentries from theme.conf to prevent broken options
# Español: Eliminar menuentries de prueba cableados de theme.conf para evitar opciones rotas
sed -i '/#MENUENTRIES/q' "$DEST_DIR/theme.conf"
sed -i 's/^hideui .*/hideui singleuser,hints/' "$DEST_DIR/theme.conf"

# Instalar iconos específicos de arranque de Pulsar OS Live
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
BOOT_ICONS_DIR="$SCRIPT_DIR/../pulsar-boot-icons"
if [ -d "$BOOT_ICONS_DIR" ]; then
    echo "📦 Instalando iconos de arranque live personalizados en rEFInd..."
    mkdir -p "$DEST_DIR/icons"
    [ -f "$BOOT_ICONS_DIR/toram.png" ] && cp -f "$BOOT_ICONS_DIR/toram.png" "$DEST_DIR/icons/os_pulsaros_toram.png"
    [ -f "$BOOT_ICONS_DIR/normal.png" ] && cp -f "$BOOT_ICONS_DIR/normal.png" "$DEST_DIR/icons/os_pulsaros_normal.png"
    [ -f "$BOOT_ICONS_DIR/normal.png" ] && cp -f "$BOOT_ICONS_DIR/normal.png" "$DEST_DIR/icons/os_pulsaros.png"
    [ -f "$BOOT_ICONS_DIR/normal.png" ] && cp -f "$BOOT_ICONS_DIR/normal.png" "$DEST_DIR/icons/os_pulsar.png"
    [ -f "$BOOT_ICONS_DIR/debug-noplymouth.png" ] && cp -f "$BOOT_ICONS_DIR/debug-noplymouth.png" "$DEST_DIR/icons/os_pulsaros_debug.png"
    [ -f "$BOOT_ICONS_DIR/old.png" ] && cp -f "$BOOT_ICONS_DIR/old.png" "$DEST_DIR/icons/os_pulsaros_old.png"
    [ -f "$BOOT_ICONS_DIR/recovery.png" ] && cp -f "$BOOT_ICONS_DIR/recovery.png" "$DEST_DIR/icons/os_recovery.png"
    [ -f "$BOOT_ICONS_DIR/recovery.png" ] && cp -f "$BOOT_ICONS_DIR/recovery.png" "$DEST_DIR/icons/os_pulsaros_recovery.png"
fi

# English: Ensure other common OS icons in the theme also have descriptive labels underneath
# Español: Asegurar que otros iconos de SO comunes tengan textos descriptivos legibles debajo
python3 -c "
import os
from PIL import Image, ImageDraw, ImageFont

icons_dir = '$DEST_DIR/icons'
if not os.path.isdir(icons_dir):
    exit(0)

font_paths = [
    '/usr/share/fonts/truetype/tahoe-sddm/SFUIText-Semibold.otf',
    '/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf',
    '/usr/share/fonts/gnu-free/FreeSansBold.otf'
]
font_path = next((p for p in font_paths if os.path.exists(p)), None)
font = ImageFont.truetype(font_path, 13) if font_path else ImageFont.load_default()

def label_icon(img_path, label):
    if not os.path.isfile(img_path):
        return
    try:
        orig = Image.open(img_path).convert('RGBA')
        bbox = orig.getbbox()
        cropped = orig.crop(bbox) if bbox else orig
        max_icon_size = 72
        cropped.thumbnail((max_icon_size, max_icon_size), Image.Resampling.LANCZOS)
        
        canvas = Image.new('RGBA', (128, 128), (0, 0, 0, 0))
        ix = (128 - cropped.width) // 2
        iy = 8 + (72 - cropped.height) // 2
        canvas.paste(cropped, (ix, iy), cropped)
        
        draw = ImageDraw.Draw(canvas)
        text_bbox = draw.textbbox((0, 0), label, font=font)
        tw = text_bbox[2] - text_bbox[0]
        tx = (128 - tw) // 2
        ty = 94
        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1), (0, 2), (1, 1), (-1, 1)]:
            draw.text((tx + dx, ty + dy), label, font=font, fill=(0, 0, 0, 230))
        draw.text((tx, ty), label, font=font, fill=(245, 245, 247, 255))
        canvas.save(img_path, 'PNG')
    except Exception as e:
        print(f'Warning labeling {img_path}: {e}')

labels_map = {
    'os_win.png': 'Windows',
    'os_win8.png': 'Windows',
    'os_win11.png': 'Windows',
    'os_windows.png': 'Windows',
    'os_arch.png': 'Arch Linux',
    'os_debian.png': 'Debian',
    'os_ubuntu.png': 'Ubuntu',
    'os_fedora.png': 'Fedora',
    'os_linux.png': 'Linux',
    'os_linux_2.png': 'Linux',
    'os_mac.png': 'macOS',
    'os_mac_alt.png': 'macOS',
    'os_mint.png': 'Linux Mint',
    'os_pop.png': 'Pop!_OS',
    'os_manjaro.png': 'Manjaro',
    'os_elementary.png': 'elementary OS',
    'os_zorin.png': 'Zorin OS',
    'os_suse.png': 'openSUSE',
    'os_opensuse.png': 'openSUSE',
    'os_gentoo.png': 'Gentoo',
    'os_freebsd.png': 'FreeBSD',
}

for icon_file, label in labels_map.items():
    p = os.path.join(icons_dir, icon_file)
    label_icon(p, label)
" 2>/dev/null || true

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Tema de rEFInd estructurado en staging."
