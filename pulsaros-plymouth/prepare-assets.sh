#!/bin/bash
# ==============================================================================
# Pulsar OS - Plymouth Theme Asset Preparer
# ==============================================================================
# Descarga e instala en el paquete el tema de Plymouth macOS-like.
# Configura el archivo daemon y oculta logotipos antiguos de Debian.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
THEME_DEST="$STAGE_DIR/usr/share/plymouth/themes/pulsar-plymouth"
mkdir -p "$THEME_DEST"

# Check if the local 'repo' directory is present in the staging folder and contains the theme configuration
# Comprobar si el directorio 'repo' local está presente en la carpeta staging y contiene la configuración del tema
if [ -f "$STAGE_DIR/repo/pulsar-plymouth.plymouth" ]; then
    echo "🎨 Copiando tema Plymouth desde el repositorio local..."
    # Copy theme assets from local repo
    # Copiar recursos del tema desde el repositorio local
    cp -r "$STAGE_DIR/repo"/* "$THEME_DEST/"
    # Remove the repo folder from staging to avoid packing it at the root of the deb package
    # Eliminar la carpeta repo de staging para evitar empaquetarla en la raíz del paquete deb
    rm -rf "$STAGE_DIR/repo"
else
    echo "⚠️ Directorio repo local vacío o no encontrado en staging. Descargando de respaldo desde Github..."
    # If the local repo folder exists but is empty (e.g. submodule not initialized), clean it up first
    # Si la carpeta repo local existe pero está vacía (ej. submódulo no inicializado), limpiarla primero
    rm -rf "$STAGE_DIR/repo"
    
    TEMP_BUILD="/tmp/pulsaros-plymouth-build"
    THEME_REPO="https://github.com/Inled-Pulsar-OS/plymouth-macoslike"
    rm -rf "$TEMP_BUILD"
    mkdir -p "$TEMP_BUILD"
    
    # Clone with depth=1 from GitHub using HTTP/1.1, low speed timeouts, and larger postBuffer
    # Clonar con depth=1 desde GitHub usando HTTP/1.1, límites de velocidad y postBuffer mayor
    git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"
    cp -r "$TEMP_BUILD/theme"/* "$THEME_DEST/"
fi

# Ensure all assets are at the root of the theme directory (not in an images/ subdirectory)
# so that the Debian/Ubuntu initramfs hooks (which only glob *.png in the root theme folder)
# can properly copy them into the ramdisk.
# Asegurar que todos los recursos estén en la raíz del tema (no en la subcarpeta images/)
# para que los hooks de initramfs de Debian/Ubuntu (que solo copian *.png de la raíz)
# puedan incluirlos correctamente en el ramdisk.
if [ -d "$THEME_DEST/images" ]; then
    echo "📂 Aplanando estructura de imágenes del tema..."
    mv "$THEME_DEST/images"/* "$THEME_DEST/" || true
    rm -rf "$THEME_DEST/images"
fi

# 2. Configurar el archivo plymouthd.conf en staging
mkdir -p "$STAGE_DIR/etc/plymouth"
cat <<EOF > "$STAGE_DIR/etc/plymouth/plymouthd.conf"
[Daemon]
Theme=pulsar-plymouth
ShowDelay=0
DeviceTimeout=8
UseFirmwareBackground=false
UseSimpledrm=false
EOF

# 3. Generar recursos de autenticación y descifrado LUKS (bullet, entry, lock, capslock)
echo "🎨 Generando interfaz de descifrado de disco (bullet dots, inputbox line, lock)..."
python3 - <<PYEOF
import cairo, math

dest = "$THEME_DEST"

def create_bullet(path, size=12):
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, size, size)
    ctx = cairo.Context(surface)
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.95)
    ctx.arc(size / 2.0, size / 2.0, (size / 2.0) - 1.5, 0, 2 * math.pi)
    ctx.fill()
    surface.write_to_png(path)

def create_entry(path, width=320, height=36, radius=18):
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, width, height)
    ctx = cairo.Context(surface)
    x, y, w, h, r = 1.0, 1.0, width - 2.0, height - 2.0, radius - 1.0
    ctx.new_sub_path()
    ctx.arc(x + r, y + r, r, math.pi, 3 * math.pi / 2)
    ctx.arc(x + w - r, y + r, r, 3 * math.pi / 2, 2 * math.pi)
    ctx.arc(x + w - r, y + h - r, r, 0, math.pi / 2)
    ctx.arc(x + r, y + h - r, r, math.pi / 2, math.pi)
    ctx.close_path()
    ctx.set_source_rgba(0.12, 0.12, 0.15, 0.85)
    ctx.fill_preserve()
    ctx.set_line_width(1.0)
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.35)
    ctx.stroke()
    surface.write_to_png(path)

def create_lock(path, size=24):
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, size, size)
    ctx = cairo.Context(surface)
    ctx.set_line_width(2.0)
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.85)
    ctx.arc(12, 9, 5, math.pi, 2 * math.pi)
    ctx.stroke()
    ctx.rectangle(6, 9, 12, 11)
    ctx.fill()
    ctx.set_source_rgba(0.1, 0.1, 0.1, 0.9)
    ctx.arc(12, 13.5, 1.5, 0, 2 * math.pi)
    ctx.fill()
    ctx.rectangle(11.2, 13.5, 1.6, 3.5)
    ctx.fill()
    surface.write_to_png(path)

def create_capslock(path, size=24):
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, size, size)
    ctx = cairo.Context(surface)
    ctx.set_source_rgba(1.0, 0.8, 0.2, 0.9)
    ctx.move_to(12, 5)
    ctx.line_to(6, 12)
    ctx.line_to(9.5, 12)
    ctx.line_to(9.5, 15)
    ctx.line_to(14.5, 15)
    ctx.line_to(14.5, 12)
    ctx.line_to(18, 12)
    ctx.close_path()
    ctx.fill()
    ctx.rectangle(9.5, 17, 5, 2)
    ctx.fill()
    surface.write_to_png(path)

create_bullet(f'{dest}/bullet.png')
create_entry(f'{dest}/entry.png')
create_lock(f'{dest}/lock.png')
create_capslock(f'{dest}/capslock.png')
PYEOF

# 4. Generar la marca de agua transparente (sin texto ni marcas de base)
echo "🎨 Generando marca de agua transparente para Plymouth..."
echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=" | base64 -d > "$THEME_DEST/watermark.png"

# 5. Reemplazar los logos y marcas de agua de Debian del sistema por transparencia
echo "Generando reemplazo de logo transparente del sistema..."
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes"
mkdir -p "$STAGE_DIR/usr/share/pixmaps"

echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=" | base64 -d > "$STAGE_DIR/usr/share/plymouth/debian-logo.png"

cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/debian-logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/pixmaps/debian-logo.png"

mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/spinner"
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/debian-spinner"
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/bgrt"

cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/spinner/watermark.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/debian-spinner/watermark.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/bgrt/watermark.png"

# Configurar el archivo de tema .plymouth
cat <<'THEME_EOF' > "$THEME_DEST/pulsar-plymouth.plymouth"
[Plymouth Theme]
Name=Pulsar OS
Description=Pulsar OS Boot Splash
ModuleName=two-step

[two-step]
Font=Cantarell 11
TitleFont=Cantarell Light 20
ImageDir=/usr/share/plymouth/themes/pulsar-plymouth
DialogHorizontalAlignment=.5
DialogVerticalAlignment=.58
TitleHorizontalAlignment=.5
TitleVerticalAlignment=.75
HorizontalAlignment=.5
VerticalAlignment=.38
Logo=header-image
Watermark=watermark
WatermarkHorizontalAlignment=.5
WatermarkVerticalAlignment=.96
Transition=none
TransitionDuration=0.0
BackgroundStartColor=0x000000
BackgroundEndColor=0x000000
ProgressBarHorizontalAlignment=.5
ProgressBarVerticalAlignment=.58
ProgressBarWidth=320
ProgressBarHeight=4
ProgressBarBackgroundColor=0x262628
ProgressBarForegroundColor=0xffffff
DialogClearsFirmwareBackground=true
MessageBelowAnimation=true

[boot-up]
UseEndAnimation=true
UseFirmwareBackground=false
SuppressMessages=false

[shutdown]
UseEndAnimation=true
UseFirmwareBackground=false

[reboot]
UseEndAnimation=true
UseFirmwareBackground=false

[updates]
UseFirmwareBackground=false
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Install Updates...
SubTitle=Please do not turn off your computer.

[system-upgrade]
UseFirmwareBackground=false
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Install Upgrades...
SubTitle=Please do not turn off your computer.

[firmware-upgrade]
UseFirmwareBackground=false
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Install Firmware-Updates...
SubTitle=Please do not turn off your computer.
THEME_EOF

# Clean up temporary build directory if it was created
# Limpiar el directorio temporal de compilación si fue creado
if [ -d "$TEMP_BUILD" ]; then
    rm -rf "$TEMP_BUILD"
fi
echo "✅ Tema Plymouth estructurado correctamente en staging."
