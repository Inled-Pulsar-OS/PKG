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
fi

# English: Always overwrite branding.desc with a correct and compliant YAML structure
# Español: Sobrescribir siempre branding.desc con una estructura YAML correcta y compatible
cat <<EOF > "$BRANDING_DEST/branding.desc"
---
componentName:         pulsaros
welcomeStyleCalamares: false
welcomeExpandingLogo:  true
stylesheet:            "stylesheet.qss"
slideshow:             "show.qml"
slideshowAPI:          2
images:
    productLogo:         "logo.png"
    productIcon:         "logo.png"
    productWelcome:      "welcome.png"
style:
   sidebarBackground:        "#1f1f1f"
   sidebarText:              "#e0e0e0"
   sidebarTextCurrent:       "#ffffff"
   sidebarBackgroundCurrent: "#0071e3"

strings:
    productName:         "PulsarOS"
    shortProductName:    "PulsarOS"
    productVersion:      "1.0"
    productUrl:          "https://inled.es"
    supportUrl:          "https://inled.es"
    knownIssuesUrl:      "https://inled.es"
    releaseNotesUrl:     "https://inled.es"
EOF

# English: Download the official Pulsar OS logo
# Español: Descargar el logo oficial de Pulsar OS
echo "📥 Descargando logo oficial de Pulsar OS..."
if ! wget -q --timeout=15 --tries=3 -O "$BRANDING_DEST/logo.png" "https://hosted.inled.es/pulsar-logo-simple-sf.png"; then
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
/* Pulsar OS Calamares Stylesheet - Apple Setup Assistant Style */

#mainApp {
    background-color: #e3e3e6;
}

#sidebarApp {
    background-color: #e3e3e6;
    min-width: 0px;
    max-width: 0px;
    width: 0px;
    border: none;
}

#sidebarApp QListWidget {
    background-color: transparent;
    border: none;
    min-width: 0px;
    max-width: 0px;
    width: 0px;
}

QWidget {
    font-family: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', Helvetica, Arial, sans-serif";
    color: #1d1d1f;
}

QLabel {
    color: #1d1d1f;
    font-size: 13px;
}

QTextEdit, QLineEdit, QComboBox, QSpinBox, QAbstractItemView {
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 8px;
    color: #1d1d1f;
    padding: 6px;
    selection-background-color: #0066cc;
    selection-color: #ffffff;
}

QTextEdit:focus, QLineEdit:focus, QComboBox:focus {
    border: 1px solid #0066cc;
}

QListView, QTreeView, QListWidget, QTreeWidget {
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 8px;
}

QPushButton {
    background-image: none;
    background-color: #ffffff;
    border: 1px solid #d2d2d7;
    border-radius: 8px;
    color: #1d1d1f;
    font-weight: 500;
    font-size: 13px;
    padding: 6px 20px;
}

QPushButton:hover {
    background-color: #f5f5f7;
    border-color: #c5c5ca;
}

QPushButton:pressed {
    background-color: #e3e3e6;
}

QPushButton:disabled {
    color: #aeaeae;
    border-color: #e3e3e6;
    background-color: #f5f5f7;
}

#nextButton {
    background-image: none;
    background-color: #0066cc;
    border: none;
    color: #ffffff;
    font-weight: bold;
}

#nextButton:hover {
    background-color: #0077ed;
}

#nextButton:pressed {
    background-color: #005bb5;
}
EOF



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

# bootloader.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/bootloader.conf"
---
efiBootLoader: "grub"
grubInstall: "grub-install"
grubMkconfig: "grub-mkconfig"
grubCfg: "/boot/grub/grub.cfg"
grubProbe: "grub-probe"
efiBootMgr: "efibootmgr"
efiBootloaderId: "pulsaros"
installEFIFallback: true
EOF

# prefill.conf
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/prefill.conf"
---
EOF

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

# Use default Debian partition.conf to dynamically choose MS-DOS for BIOS and GPT for UEFI boot schemes.
# Usar el partition.conf predeterminado de Debian para elegir dinámicamente MS-DOS en BIOS y GPT en UEFI.

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

# packages.conf (configura la instalación automatizada de drivers y firmware de Pulsar OS, y desinstala calamares y recovery)
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/packages.conf"
---
backend: apt

operations:
  - install:
      - firmware-linux
      - firmware-linux-nonfree
      - firmware-misc-nonfree
      - firmware-iwlwifi
      - firmware-realtek
      - firmware-atheros
      - firmware-brcm80211
      - intel-microcode
      - amd64-microcode
      - firmware-amd-graphics
  # Use try_remove instead of remove to prevent installation failure if these packages are not found.
  # Usar try_remove en lugar de remove para evitar fallos de instalación si estos paquetes no se encuentran.
  # (pulsaros-recovery se mantiene en el sistema instalado y se convierte en herramienta de recuperación / pulsaros-recovery is kept on installed system and becomes a recovery tool)
  - try_remove:
      - calamares
      - calamares-settings-debian
      - pulsaros-calamares
EOF


# shellprocess@recovery.conf
# English: Modifies the recovery launcher in the final installed system (Name to PulsarOS Recovery and Icon to system-backup)
# Español: Modifica el lanzador de recovery en el sistema final instalado (Nombre a PulsarOS Recovery e Icono a system-backup)
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/shellprocess@recovery.conf"
---
dontChroot: false
timeout: 30
script:
    - "if [ -f /usr/share/applications/pulsaros-recovery.desktop ]; then sed -i 's/^Name=.*/Name=PulsarOS Recovery/' /usr/share/applications/pulsaros-recovery.desktop && sed -i 's/^Icon=.*/Icon=system-backup/' /usr/share/applications/pulsaros-recovery.desktop; fi"
EOF


# shellprocess@refind.conf
# English: Installs and configures rEFInd with macOS theme in the final system
# Español: Instala y configura rEFInd con el tema macOS en el sistema final
cat <<EOF > "$CALAMARES_CONFIGS_DEST/modules/shellprocess@refind.conf"
---
dontChroot: false
timeout: 120
script:
    - "echo '⚙️ Instalando rEFInd... / Installing rEFInd...'"
    - "refind-install --yes"
    - "echo '🎨 Instalando tema macOS para rEFInd... / Installing macOS theme...'"
    - "mkdir -p /boot/efi/EFI/refind/themes"
    - "rm -rf /boot/efi/EFI/refind/themes/rEFInd-Regular-Dark"
    - "if [ -d /usr/share/refind/themes/rEFInd-Regular-Dark ]; then cp -r /usr/share/refind/themes/rEFInd-Regular-Dark /boot/efi/EFI/refind/themes/; fi"
    - "REFIND_CONF=\"/boot/efi/EFI/refind/refind.conf\""
    - "if [ -f \"\$REFIND_CONF\" ]; then sed -i 's/^#enable_mouse/enable_mouse/' \"\$REFIND_CONF\" && sed -i 's/^enable_mouse.*/enable_mouse/' \"\$REFIND_CONF\" && grep -q \"^enable_mouse\" \"\$REFIND_CONF\" || echo \"enable_mouse\" >> \"\$REFIND_CONF\"; grep -q \"themes/rEFInd-Regular-Dark/theme.conf\" \"\$REFIND_CONF\" || echo \"include themes/rEFInd-Regular-Dark/theme.conf\" >> \"\$REFIND_CONF\"; fi"
EOF


# prefill module descriptor and script
mkdir -p "$STAGE_DIR/usr/share/calamares/modules/prefill"
cat <<EOF > "$STAGE_DIR/usr/share/calamares/modules/prefill/module.desc"
---
type: "job"
name: "prefill"
interface: "python"
script: "main.py"
EOF

cat <<'EOF' > "$STAGE_DIR/usr/share/calamares/modules/prefill/main.py"
import libcalamares
import json
import os

def obscure(s):
    result = []
    for c in s:
        code = ord(c)
        if 0x00 < code < 0x20:
            code = 0x20 - code
        elif 0x20 <= code < 0x7f:
            code = 0x7f - (code - 0x20)
        elif 0x80 <= code < 0x100:
            code = 0x100 - (code - 0x80)
        result.append(chr(code))
    return "".join(result)

def run():
    json_path = "/tmp/recovery-settings.json"
    if not os.path.exists(json_path):
        libcalamares.utils.debug("Prefill: /tmp/recovery-settings.json not found")
        return None
        
    try:
        with open(json_path, "r") as f:
            data = json.load(f)
            
        # Timezone/Locale:
        libcalamares.globalstorage.insert("timezone", data.get("timezone", "Europe/Madrid"))
        
        # Keyboard:
        libcalamares.globalstorage.insert("keyboardLayout", data.get("keyboardLayout", "es"))
        libcalamares.globalstorage.insert("keyboardVariant", "")
        
        # Users:
        username = data.get("username", "pepe")
        password = data.get("password", "")
        hostname = data.get("hostname", "pulsaros-pc")
        root_pwd = data.get("rootPassword", password)
        
        libcalamares.globalstorage.insert("username", username)
        libcalamares.globalstorage.insert("fullName", data.get("fullName", username))
        libcalamares.globalstorage.insert("userRealName", data.get("fullName", username))
        libcalamares.globalstorage.insert("hostname", hostname)
        libcalamares.globalstorage.insert("password", obscure(password))
        libcalamares.globalstorage.insert("rootPassword", obscure(root_pwd))
        libcalamares.globalstorage.insert("autologinUser", username if data.get("autologin", True) else "")
        
        libcalamares.utils.debug("Prefill: Successfully injected settings into Calamares GlobalStorage!")
    except Exception as e:
        libcalamares.utils.debug(f"Prefill: Error loading settings: {e}")
        
    return None
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
- id:       recovery
  module:   shellprocess
  config:   shellprocess@recovery.conf
- id:       refind
  module:   shellprocess
  config:   shellprocess@refind.conf
- id:       prefill
  module:   prefill
  config:   prefill.conf

sequence:
- show:
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
  - shellprocess@recovery
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
# Lanzar calamares usando pkexec o sudo env en su defecto
# Launch calamares using pkexec or sudo env as fallback
if command -v pkexec >/dev/null 2>&1; then
    pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" calamares "$@"
else
    sudo env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" calamares "$@" || calamares "$@"
fi
EOF

# Ensure launch script is executable
# Asegurar que el script de lanzamiento sea ejecutable
chmod 755 "$STAGE_DIR/usr/local/bin/launch-calamares"

# Note: Automatic autostart on the live CD is now managed by pulsaros-recovery.
# Calamares will be launched by the recovery application when needed.
echo "✅ Calamares configurado en staging."
