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
else
    echo "⚠️ Advertencia: No se encontró el branding en $LOCAL_BRANDING. Usando fallback funcional..."
    
    cat <<EOF > "$BRANDING_DEST/branding.desc"
---
componentName:  pulsaros
welcomeStyleCalamares:   false
welcomeExpandingLogo:   true
strings:
    productName:         PulsarOS
    shortProductName:    PulsarOS
    productVersion:      1.0
    productUrl:          https://inled.es
images:
    productLogo:         "logo.png"
    productIcon:         "logo.png"
    productWelcome:      "welcome.png"
style:
   sidebarBackground:    "#1f1f1f"
   sidebarText:          "#e0e0e0"
   sidebarTextCurrent:       "#1f1f1f"
   sidebarBackgroundCurrent: "#0a84ff"
slideshowAPI: 2
EOF
    # Crear placeholders para que no falle Calamares si no encuentra imágenes
    if command -v convert >/dev/null 2>&1; then
        convert -size 64x64 xc:blue "$BRANDING_DEST/logo.png"
        convert -size 400x200 xc:darkgrey "$BRANDING_DEST/welcome.png"
    else
        touch "$BRANDING_DEST/logo.png"
        touch "$BRANDING_DEST/welcome.png"
    fi
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
# 3. LANZADOR Y AUTO-ARRANQUE EN EL LIVE
# ==============================================================================
echo "🖥️ Configurando lanzador y auto-arranque..."
mkdir -p "$STAGE_DIR/usr/local/bin"
cat <<'EOF' > "$STAGE_DIR/usr/local/bin/launch-calamares"
#!/bin/bash
# Permitir conexiones X11 locales para root en Wayland
xhost +local:root > /dev/null 2>&1 || true
# Lanzar calamares preservando el entorno
sudo -E DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" calamares "$@"
EOF

# Auto-arranque
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

echo "✅ Calamares configurado en staging."
