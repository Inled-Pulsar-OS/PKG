#!/bin/bash
# ==============================================================================
# Pulsar OS - Calamares Config Asset Preparer
# ==============================================================================
# Genera y estructura todas las configuraciones del instalador Calamares.
# Copia el branding local si existe, u ofrece un fallback de Pulsar OS.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
BRANDING_DEST="$STAGE_DIR/usr/share/calamares/branding/pulsaros"
CALAMARES_CONFIGS_DEST="$STAGE_DIR/etc/calamares"

mkdir -p "$BRANDING_DEST"
mkdir -p "$CALAMARES_CONFIGS_DEST/modules"

# ==============================================================================
# 1. INTENTAR COPIAR BRANDING LOCAL O USAR FALLBACK
# ==============================================================================
LOCAL_BRANDING="/home/jaime/Documentos/pulsaros-base/calamares/etc/calamares/branding/pearOS"

if [ -d "$LOCAL_BRANDING" ]; then
    echo "📥 Copiando branding de Calamares desde: $LOCAL_BRANDING"
    cp -r "$LOCAL_BRANDING"/* "$BRANDING_DEST/"
    
    # Ajustar branding.desc para PulsarOS
    sed -i 's/^componentName:.*/componentName:  pulsaros/' "$BRANDING_DEST/branding.desc"
    sed -i 's/pearOS NiceC0re/PulsarOS/g' "$BRANDING_DEST/branding.desc"
    sed -i 's/pearOS/PulsarOS/g' "$BRANDING_DEST/branding.desc"
    sed -i 's/version:             26.03/version:             1.0/g' "$BRANDING_DEST/branding.desc"
    sed -i 's/shortVersion:        26.3/shortVersion:        1.0/g' "$BRANDING_DEST/branding.desc"
    
    # English: Remove old occurrences of these variables to put them at top-level
    # Español: Eliminar ocurrencias antiguas de estas variables para ponerlas a nivel raíz
    sed -i '/productName:/d' "$BRANDING_DEST/branding.desc"
    sed -i '/shortProductName:/d' "$BRANDING_DEST/branding.desc"
    sed -i '/productVersion:/d' "$BRANDING_DEST/branding.desc"
    sed -i '/productUrl:/d' "$BRANDING_DEST/branding.desc"
    sed -i '/stylesheet:/d' "$BRANDING_DEST/branding.desc"
    
    # English: Insert top-level variables right after the YAML frontmatter marker '---'
    # Español: Insertar variables a nivel raíz justo después del marcador YAML '---'
    sed -i '/---/a \
productName:         PulsarOS\
shortProductName:    PulsarOS\
productVersion:      1.0\
productUrl:          https://inled.es\
stylesheet:          "stylesheet.qss"' "$BRANDING_DEST/branding.desc"
else
    echo "⚠️ Advertencia: No se encontró el branding en $LOCAL_BRANDING. Usando fallback funcional..."
    
    cat <<EOF > "$BRANDING_DEST/branding.desc"
---
componentName:         pulsaros
productName:           PulsarOS
shortProductName:      PulsarOS
productVersion:        1.0
productUrl:            https://inled.es
stylesheet:            "stylesheet.qss"
welcomeStyleCalamares: false
welcomeExpandingLogo:  true
images:
    productLogo:         "logo.png"
    productIcon:         "logo.png"
    productWelcome:      "welcome.png"
style:
   sidebarBackground:        "#1f1f1f"
   sidebarText:              "#e0e0e0"
   sidebarTextCurrent:       "#ffffff"
   sidebarBackgroundCurrent: "#0071e3"
slideshowAPI: 2
EOF
fi

# English: Download the official Pulsar OS logo
# Español: Descargar el logo oficial de Pulsar OS
echo "📥 Descargando logo oficial de Pulsar OS..."
if ! wget -q -O "$BRANDING_DEST/logo.png" "https://hosted.inled.es/pulsar-logo-simple-sf.png"; then
    echo "⚠️ No se pudo descargar el logo desde internet, usando un fallback local..."
    if command -v convert >/dev/null 2>&1; then
        convert -size 64x64 xc:blue "$BRANDING_DEST/logo.png"
    else
        touch "$BRANDING_DEST/logo.png"
    fi
fi

# English: Create welcome placeholder if it doesn't exist
# Español: Crear marcador de posición welcome si no existe
if [ ! -f "$BRANDING_DEST/welcome.png" ]; then
    if command -v convert >/dev/null 2>&1; then
        convert -size 400x200 xc:darkgrey "$BRANDING_DEST/welcome.png"
    else
        touch "$BRANDING_DEST/welcome.png"
    fi
fi

# English: Create a beautiful macOS-like stylesheet for Calamares to fix the selected sidebar item styling
# Español: Crear una hermosa hoja de estilo tipo macOS para Calamares para solucionar el estilo del elemento seleccionado de la barra lateral
cat <<EOF > "$BRANDING_DEST/stylesheet.qss"
/* Pulsar OS Calamares Stylesheet - Modern Dark Theme */

#mainApp {
    background-color: #1e1e1e;
}

#sidebarApp {
    background-color: #1e1e1e;
    min-width: 220px;
}

#sidebarApp QListWidget {
    background-color: #1e1e1e;
    border: none;
}

#sidebarApp QListWidget::item {
    color: #a0a0a0;
    padding: 12px 16px;
    font-size: 13px;
    font-weight: 500;
}

#sidebarApp QListWidget::item:selected {
    background-color: #0071e3;
    color: #ffffff;
    border-radius: 6px;
}

#sidebarApp QListWidget::item:hover:!selected {
    background-color: #2c2c2c;
    color: #ffffff;
    border-radius: 6px;
}
EOF

# English: Ensure slideshow keys are present in branding.desc to prevent Calamares startup crash
# Español: Asegurar que las claves de la presentación de diapositivas estén en branding.desc para evitar el fallo de arranque
if [ -f "$BRANDING_DEST/branding.desc" ]; then
    # Eliminar líneas anteriores para evitar duplicaciones
    sed -i '/^slideshow:/d' "$BRANDING_DEST/branding.desc"
    sed -i '/^slideshowAPI:/d' "$BRANDING_DEST/branding.desc"
    echo "slideshow: \"show.qml\"" >> "$BRANDING_DEST/branding.desc"
    echo "slideshowAPI: 2" >> "$BRANDING_DEST/branding.desc"
fi

# English: Create a fallback slideshow QML file if not present
# Español: Crear un archivo QML de presentación de diapositivas de respaldo si no existe
if [ ! -f "$BRANDING_DEST/show.qml" ]; then
    echo "Generating show.qml slideshow fallback..."
    cat <<'EOF' > "$BRANDING_DEST/show.qml"
import QtQuick 2.0
import calamares.slideshow 1.0

Presentation {
    id: presentation

    Timer {
        id: advanceTimer
        interval: 5000
        running: presentation.activatedInCalamares
        repeat: true
        onTriggered: presentation.goToNextSlide()
    }

    Slide {
        Text {
            anchors.centerIn: parent
            text: "Welcome to Pulsar OS!"
            font.pixelSize: 22
            color: "#ffffff"
        }
    }

    Slide {
        Text {
            anchors.centerIn: parent
            text: "Setting up a secure and fast environment..."
            font.pixelSize: 18
            color: "#e0e0e0"
        }
    }
}
EOF
fi

# ==============================================================================
# 2. CONFIGURAR MÓDULOS Y SETTINGS
# ==============================================================================
echo "⚙️ Configurando módulos de Calamares..."

# welcome.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/welcome.conf"
---
showSupportUrl:         false
showKnownIssuesUrl:     false
showReleaseNotesUrl:    false
showRunCalamaresUrl:    false

requirements:
    requiredStorage:    5.0
    requiredRam:        1.0
    internetCheckUrl:   http://google.com
    check:
        - storage
        - ram
        - power
        - internet
        - root
    required:
        - root

geoip:
    style:    "none"
EOF

# removeuser-live.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/removeuser-live.conf"
---
username: live
EOF

# removeuser-jaime.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/removeuser-jaime.conf"
---
username: jaime
EOF

# users.conf fallback (ajustando hostname por defecto a pulsaros)
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/users.conf"
---
makeuproot: true
defaultGroups:
    - docker
    - sudo
    - users
    - lpadmin
    - sambashare
autologinUserWithWelcome: true
writeUsersPageToDummy: false
userShell: /bin/bash
EOF

# settings.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/settings.conf"
---
modules-search: [ local, /usr/lib/x86_64-linux-gnu/calamares/modules, /usr/share/calamares/modules ]

instances:
- id:       debian
  module:   packages
  config:   packages.conf
- id:       live
  module:   removeuser
  config:   removeuser-live.conf
- id:       jaime
  module:   removeuser
  config:   removeuser-jaime.conf

sequence:
- show:
  - welcome
  - locale
  - keyboard
  - partition
  - users
  - summary
- exec:
  - partition
  - mount
  - unpackfs
  - machineid
  - fstab
  - locale
  - keyboard
  - localecfg
  - users
  - displaymanager
  - networkcfg
  - hwclock
  - services-systemd
  - packages
  - grubcfg
  - removeuser@live
  - removeuser@jaime
  - bootloader
  - umount
- show:
  - finished

branding: pulsaros
prompt-install: true
dont-chroot: false
EOF

# ==============================================================================
# 3. LANZADOR Y AUTO-ARRANQUE EN EL LIVE / LAUNCHER AND AUTOSTART IN LIVE
# ==============================================================================
echo "🖥️ Configurando lanzador y auto-arranque..."
mkdir -p "$STAGE_DIR/usr/local/bin"
cat <<'EOF' > "$STAGE_DIR/usr/local/bin/launch-calamares"
#!/bin/bash
# Permitir conexiones X11 locales para root en Wayland
# Allow local X11 connections for root in Wayland
xhost +local:root > /dev/null 2>&1 || true
# Forzar a Qt a usar XWayland/X11 para evitar fallos de conexión gráfica como root en Wayland
# Force Qt to use X11/XWayland to avoid graphical connection failures as root under Wayland
export QT_QPA_PLATFORM=xcb
# Lanzar calamares preservando el entorno
# Launch calamares preserving the environment
sudo -E DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" calamares "$@"
EOF

# Ensure launch script is executable
# Asegurar que el script de lanzamiento sea ejecutable
chmod 755 "$STAGE_DIR/usr/local/bin/launch-calamares"

# User-level autostart (for skeleton / user homes)
# Auto-arranque a nivel de usuario (para esqueleto / nuevos usuarios)
mkdir -p "$STAGE_DIR/etc/skel/.config/autostart"
cat <<EOF > "$STAGE_DIR/etc/skel/.config/autostart/calamares.desktop"
[Desktop Entry]
Type=Application
Name=Install PulsarOS
GenericName=System Installer
Exec=/usr/local/bin/launch-calamares
Icon=calamares
Terminal=false
Categories=Qt;System;
X-GNOME-Autostart-enabled=true
EOF
chmod 644 "$STAGE_DIR/etc/skel/.config/autostart/calamares.desktop"

# Global-level autostart (ensures it launches for already created live user)
# Auto-arranque a nivel global (asegura que arranque para el usuario live ya creado)
mkdir -p "$STAGE_DIR/etc/xdg/autostart"
cp "$STAGE_DIR/etc/skel/.config/autostart/calamares.desktop" "$STAGE_DIR/etc/xdg/autostart/"
chmod 644 "$STAGE_DIR/etc/xdg/autostart/calamares.desktop"

echo "✅ Calamares configurado en staging."
