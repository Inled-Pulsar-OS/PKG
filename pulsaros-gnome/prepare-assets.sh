#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome Shell Extensions Asset Preparer
# ==============================================================================
# Descarga e instala localmente las extensiones GNOME Shell seleccionadas
# dentro de la estructura temporal del paquete pulsaros-gnome.
# Evita descargas durante el bootstrap y estabiliza las dependencias.
# ==============================================================================

set -e

STAGE_DIR="${1:-/tmp/pulsaros-stage}"
STAGE_DIR="$(realpath -m "$STAGE_DIR")" 2>/dev/null || STAGE_DIR="/tmp/pulsaros-stage"

# --- DYNAMIC GNOME VERSION DETECTION / DETECCIÓN DINÁMICA DE LA VERSIÓN DE GNOME ---
# We try to detect the installed GNOME Shell version to ensure we download compatible extension zips.
# Intentamos detectar la versión instalada de GNOME Shell para descargar zips de extensión compatibles.
GNOME_VER=""

# 1. Search in target/base chroot dpkg or pacman databases
POSSIBLE_DPKG_STATUS=(
    "$STAGE_DIR/var/lib/dpkg/status"
    "../../ISO/build/rootfs-target/var/lib/dpkg/status"
    "../../ISO/build/rootfs-base/var/lib/dpkg/status"
    "../../build/rootfs-target/var/lib/dpkg/status"
    "../../build/rootfs-base/var/lib/dpkg/status"
)

POSSIBLE_PACMAN_DB=(
    "$STAGE_DIR/var/lib/pacman/local"
    "../../ISO/build/rootfs-target-stable-arch-minimal-refind/var/lib/pacman/local"
    "../../ISO/build/rootfs-target-stable-arch-minimal/var/lib/pacman/local"
    "../../ISO/build/rootfs-base-stable-arch-minimal/var/lib/pacman/local"
    "../../ISO/build/rootfs-target-stable-arch/var/lib/pacman/local"
    "../../ISO/build/rootfs-base-stable-arch/var/lib/pacman/local"
    "../../build/rootfs-target-stable-arch-minimal/var/lib/pacman/local"
    "../../build/rootfs-base-stable-arch-minimal/var/lib/pacman/local"
    "../../build/rootfs-target-stable-arch/var/lib/pacman/local"
    "../../build/rootfs-base-stable-arch/var/lib/pacman/local"
)

for db_dir in "${POSSIBLE_PACMAN_DB[@]}"; do
    if [ -d "$db_dir" ]; then
        for pkg_dir in "$db_dir"/gnome-shell-[0-9]*; do
            if [ -d "$pkg_dir" ]; then
                ver_str=$(basename "$pkg_dir" | sed 's/^gnome-shell-//' | cut -d. -f1 | sed 's/^[0-9]*://')
                if [ -n "$ver_str" ]; then
                    GNOME_VER="$ver_str"
                    echo "🔍 [ES] Detectada versión de GNOME Shell en Pacman DB ($pkg_dir): $GNOME_VER"
                    echo "🔍 [EN] Detected GNOME Shell version in Pacman DB ($pkg_dir): $GNOME_VER"
                    break 2
                fi
            fi
        done
    fi
done

if [ -z "$GNOME_VER" ]; then
    for status_file in "${POSSIBLE_DPKG_STATUS[@]}"; do
        if [ -f "$status_file" ]; then
            ver_str=$(grep -A 10 "Package: gnome-shell$" "$status_file" | grep "^Version:" | cut -d' ' -f2 || true)
            if [ -n "$ver_str" ]; then
                GNOME_VER=$(echo "$ver_str" | cut -d'.' -f1 | cut -d'-' -f1)
                echo "🔍 [ES] Detectada versión de GNOME Shell en chroot ($status_file): $GNOME_VER"
                echo "🔍 [EN] Detected GNOME Shell version in chroot ($status_file): $GNOME_VER"
                break
            fi
        fi
    done
fi

# 2. Check TARGET_DISTRO or host environment if chroot is not found
if [ -z "$GNOME_VER" ]; then
    for env_file in "../../configs/env.sh" "../configs/env.sh" "configs/env.sh"; do
        if [ -f "$env_file" ]; then
            source "$env_file"
            break
        fi
    done

    IS_ARCH=false
    if [ "$TARGET_DISTRO" = "arch" ] || [ "$DISTRO" = "arch" ] || [ -f /etc/arch-release ] || [ -n "$PACMAN" ]; then
        IS_ARCH=true
    fi

    if [ "$IS_ARCH" = true ]; then
        echo "🔍 [ES] Detectando versión de GNOME para Arch Linux..."
        echo "🔍 [EN] Detecting GNOME version for Arch Linux..."
        if command -v pacman >/dev/null 2>&1; then
            pacman_ver=$(pacman -Si gnome-shell 2>/dev/null | grep -i '^vers' | awk '{print $NF}' | sed 's/^[0-9]*://' | cut -d. -f1 || true)
            if [ -n "$pacman_ver" ]; then
                GNOME_VER="$pacman_ver"
                echo "🌐 [ES] Pacman devolvió versión de GNOME: $GNOME_VER"
                echo "🌐 [EN] Pacman returned GNOME version: $GNOME_VER"
            fi
        fi
        if [ -z "$GNOME_VER" ] && command -v curl >/dev/null 2>&1; then
            arch_api_ver=$(curl --max-time 3 -s "https://archlinux.org/packages/extra/x86_64/gnome-shell/json/" 2>/dev/null | grep -o '"pkgver": "[^"]*"' | cut -d'"' -f4 | cut -d. -f1 || true)
            if [ -n "$arch_api_ver" ]; then
                GNOME_VER="$arch_api_ver"
                echo "🌐 [ES] API Arch Linux devolvió versión de GNOME: $GNOME_VER"
                echo "🌐 [EN] Arch Linux API returned GNOME version: $GNOME_VER"
            fi
        fi
        if [ -z "$GNOME_VER" ]; then
            GNOME_VER="51"
        fi
    else
        DEBIAN_VERSION="${DEBIAN_VERSION:-trixie}"
        echo "🔍 [ES] No se encontró chroot. Decidiendo según versión de Debian: $DEBIAN_VERSION"
        echo "🔍 [EN] Chroot not found. Inferring version from Debian suite: $DEBIAN_VERSION"

        if command -v curl >/dev/null 2>&1; then
            suite_indicator=""
            case "$DEBIAN_VERSION" in
                trixie) suite_indicator="deb13" ;;
                bookworm) suite_indicator="deb12" ;;
                sid|unstable) suite_indicator="unstable" ;;
            esac

            if [ -n "$suite_indicator" ]; then
                madison_ver=$(curl --max-time 3 -s "https://api.ftp-master.debian.org/madison?package=gnome-shell&text=on" | grep "$suite_indicator" | head -n 1 | cut -d'|' -f2 | tr -d '[:space:]' || true)
                if [ -n "$madison_ver" ]; then
                    GNOME_VER=$(echo "$madison_ver" | cut -d'.' -f1 | cut -d'-' -f1)
                    echo "🌐 [ES] API Debian devolvió versión de GNOME: $GNOME_VER"
                    echo "🌐 [EN] Debian API returned GNOME version: $GNOME_VER"
                fi
            fi
        fi
    fi

    # 3. Check host gnome-shell version if available
    if [ -z "$GNOME_VER" ] && command -v gnome-shell >/dev/null 2>&1; then
        GNOME_VER=$(gnome-shell --version | grep -o '[0-9]\+' | head -n 1)
        echo "🔍 [ES] Detectada versión de GNOME Shell en host: $GNOME_VER"
        echo "🔍 [EN] Detected GNOME Shell version on host: $GNOME_VER"
    fi

    # 4. Final static fallback
    if [ -z "$GNOME_VER" ]; then
        GNOME_VER="48"
        echo "🔍 [ES] Usando fallback estático de GNOME: $GNOME_VER"
        echo "🔍 [EN] Using static GNOME fallback: $GNOME_VER"
    fi
fi

EGO_EXTENSIONS=(
    "blur-my-shell@aunetx"
    "wiggle@mechtifs"
    "compiz-alike-magic-lamp-effect@hermes83.github.com"
    "just-perfection-desktop@just-perfection"
    "notification-position@drugo.dev"
    "no-overview@fthx"
    "ding@rastersoft.com"
    "gsconnect@andyholmes.github.io"
    "gnome-ui-tune@itstime.tech"
    "appindicatorsupport@rgcjonas.gmail.com"
)

echo "🧩 Descargando extensiones de GNOME Shell desde Extensions.gnome.org (EGO)..."

for uuid in "${EGO_EXTENSIONS[@]}"; do
    echo "Descargando: $uuid..."

    # --- DOWNLOAD WITH BACKWARD FALLBACK / DESCARGA CON FALLBACK HACIA ATRÁS ---
    # We query EGO starting from the target GNOME version and go backward if not found.
    # Consultamos en EGO empezando por la versión de GNOME objetivo y retrocedemos si no se encuentra.
    download_path="null"
    current_ver=$GNOME_VER

    while [ "$download_path" = "null" ] || [ -z "$download_path" ]; do
        info_url="https://extensions.gnome.org/extension-info/?uuid=${uuid}&shell_version=${current_ver}"
        if command -v jq >/dev/null 2>&1; then
            download_path=$(curl -s "$info_url" | jq -r '.download_url')
        else
            download_path=$(curl -s "$info_url" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('download_url') or '')" 2>/dev/null || echo "null")
        fi

        if [ "$download_path" != "null" ] && [ -n "$download_path" ]; then
            if [ "$current_ver" -ne "$GNOME_VER" ]; then
                echo "⚠️ [ES] Usando fallback compatible con GNOME $current_ver para $uuid"
                echo "⚠️ [EN] Using compatible fallback for GNOME $current_ver for $uuid"
            fi
            break
        fi

        # Stop at version 40 to avoid checking infinite/ancient versions
        # Detenerse en la versión 40 para evitar comprobar versiones antiguas/infinitas
        if [ "$current_ver" -le 40 ]; then
            break
        fi
        current_ver=$((current_ver - 1))
    done

    if [ "$download_path" != "null" ] && [ -n "$download_path" ]; then
        full_url="https://extensions.gnome.org${download_path}"
        tmp_zip="/tmp/${uuid}.zip"

        if curl --connect-timeout 15 --max-time 60 --retry 3 -L -s -o "$tmp_zip" "$full_url"; then
            dest_dir="$STAGE_DIR/usr/share/gnome-shell/extensions/${uuid}"
            mkdir -p "$dest_dir"
            if unzip -q -o "$tmp_zip" -d "$dest_dir" 2>/dev/null; then
                echo "✅ Extensión $uuid lista."
            else
                echo "⚠️ Error al descomprimir $uuid (posible zip corrupto o límite de API EGO)"
            fi
            rm -f "$tmp_zip"
        fi
    else
        echo "⚠️ [ES] No se encontró ninguna versión compatible en EGO para $uuid"
        echo "⚠️ [EN] No compatible version found on EGO for $uuid"
    fi
done

# --- DECLARATIVE PATCHES FOR BLUR-MY-SHELL ---
BMS_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/blur-my-shell@aunetx"
if [ -d "$BMS_DIR" ]; then
    echo "🔧 [ES] Aplicando parches declarativos para Blur My Shell..."
    echo "🔧 [EN] Applying declarative patches for Blur My Shell..."

    BMS_TARGETS="$BMS_DIR/components/popup/targets.js"
    if [ -f "$BMS_TARGETS" ]; then
        python3 -c "
import sys
f = sys.argv[1]
with open(f, 'r') as fp: c = fp.read()
target_needle = '''    find_actor(actor) {
        if (this.actor.is_internal_actor(actor) || !this.actor.watch_actor(actor))
            return [];

        const targets = this.create_targets();
        const delegate = this.actor.get_actor_delegate(actor);'''
replacement = '''    find_actor(actor) {
        if (this.actor.is_internal_actor(actor) || !this.actor.watch_actor(actor))
            return [];

        const delegate = this.actor.get_actor_delegate(actor);
        const isExcluded = a => {
            if (!a) return false;
            return this.has_any_style_class(a, [
                \'mac-nc-datemenu\', \'mac-cc-quick-settings\', \'mac-nc-sidebar\',
            ]);
        };
        if (isExcluded(actor) || isExcluded(delegate?.box) || isExcluded(delegate?.actor))
            return [];

        let parentCheck = actor?.get_parent ? actor.get_parent() : null;
        while (parentCheck) {
            if (isExcluded(parentCheck))
                return [];
            parentCheck = parentCheck.get_parent ? parentCheck.get_parent() : null;
        }

        const targets = this.create_targets();'''
if 'mac-nc-datemenu' not in c and target_needle in c:
    c = c.replace(target_needle, replacement, 1)
    with open(f, 'w') as fp: fp.write(c)
" "$BMS_TARGETS"
    fi

    BMS_DUMMY="$BMS_DIR/conveniences/dummy_pipeline.js"
    if [ -f "$BMS_DUMMY" ]; then
        python3 -c "
import sys
f = sys.argv[1]
with open(f, 'r') as fp: c = fp.read()
if 'this.corner_effect = null' not in c:
    c = c.replace('this.effect = null;', 'this.effect = null;\n        this.corner_effect = null;')
    c = c.replace('if (this.actor)\n            this.actor.add_effect(this.effect);', 'if (this.actor) {\n            this.actor.add_effect(this.effect);\n            if (this.corner_effect)\n                this.actor.add_effect(this.corner_effect);\n        }')
    c = c.replace('this.effect = this.effects_manager.new_native_dynamic_gaussian_blur_effect(params);', 'this.effect = this.effects_manager.new_native_dynamic_gaussian_blur_effect(params);\n\n        if (!this.effect.constructor.supports_corner_radius) {\n            this.corner_effect = this.effects_manager.new_corner_effect({\n                radius: this.corner_radius_getter(),\n            });\n        }')
    c = c.replace('this.effect.unscaled_corner_radius = this.corner_radius_getter();', 'this.effect.unscaled_corner_radius = this.corner_radius_getter();\n        if (this.corner_effect)\n            this.corner_effect.radius = this.corner_radius_getter();')
    c = c.replace('this.effect?.queue_repaint();', 'this.effect?.queue_repaint();\n        this.corner_effect?.queue_repaint();')
    c = c.replace('if (this.effect)\n            this.effects_manager.remove(this.effect);\n        this.effect = null;', 'if (this.effect)\n            this.effects_manager.remove(this.effect);\n        this.effect = null;\n        if (this.corner_effect)\n            this.effects_manager.remove(this.corner_effect);\n        this.corner_effect = null;')
    with open(f, 'w') as fp: fp.write(c)
" "$BMS_DUMMY"
    fi

    BMS_CSS="$BMS_DIR/styles/popup/base.css"
    if [ -f "$BMS_CSS" ]; then
        sed -i 's/border-radius: 0 0 13px 13px;/border-radius: 12px;/g' "$BMS_CSS"
        sed -i 's/border-radius: 0 0 12px 12px;/border-radius: 8px;/g' "$BMS_CSS"
    fi
fi

# --- DECLARATIVE PATCHES FOR APPINDICATOR SUPPORT ---
APPINDICATOR_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/appindicatorsupport@rgcjonas.gmail.com"
if [ -d "$APPINDICATOR_DIR" ]; then
    echo "🔧 [ES] Aplicando parches para AppIndicator Support (menú click derecho)..."
    echo "🔧 [EN] Applying patches for AppIndicator Support (right-click menu)..."

    APPIND_ICON="$APPINDICATOR_DIR/indicatorStatusIcon.js"
    if [ -f "$APPIND_ICON" ]; then
        python3 -c "
import sys
f = sys.argv[1]
with open(f, 'r') as fp: c = fp.read()
target = '''        if (event.get_button() === Clutter.BUTTON_SECONDARY) {
            this.menu.toggle();
            return Clutter.EVENT_PROPAGATE;
        }'''
replacement = '''        if (event.get_button() === Clutter.BUTTON_SECONDARY) {
            if (this.menu.numMenuItems) {
                this.menu.toggle();
            } else if (this._indicator?.secondaryActivate) {
                this._indicator.secondaryActivate(event.get_time(), ...event.get_coords());
            } else {
                this.menu.toggle();
            }
            return Clutter.EVENT_STOP;
        }'''
if target in c:
    c = c.replace(target, replacement, 1)
    with open(f, 'w') as fp: fp.write(c)
" "$APPIND_ICON"
    fi
fi

# --- DECLARATIVE PATCHES FOR NOTIFICATION POSITION ---
# GNOME 51 removed St.BoxLayout's :vertical property in favor of :orientation.
NOTIF_POS_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/notification-position@drugo.dev"
if [ -d "$NOTIF_POS_DIR" ]; then
    echo "🔧 [ES] Aplicando parches para Notification Position (St.BoxLayout GNOME 51)..."
    echo "🔧 [EN] Applying patches for Notification Position (St.BoxLayout GNOME 51)..."
    NOTIF_POS_JS="$NOTIF_POS_DIR/extension.js"
    if [ -f "$NOTIF_POS_JS" ]; then
        python3 -c "
import sys
f = sys.argv[1]
with open(f, 'r') as fp: c = fp.read()
c = c.replace(
    'new St.BoxLayout({ vertical: true, x_expand: true })',
    'new St.BoxLayout({ orientation: Clutter.Orientation.VERTICAL, x_expand: true })')
with open(f, 'w') as fp: fp.write(c)
" "$NOTIF_POS_JS"
    fi
fi

# INSTALL PULSAR DOCK (OUR DASH-TO-DOCK FORK) REPLACING UPSTREAM DASH TO DOCK
# INSTALAR PULSAR DOCK (NUESTRO FORK DE DASH-TO-DOCK) EN LUGAR DEL ORIGINAL
# ==============================================================================
# The fork is built from source because stylesheet.css, gschemas.compiled and
# the .mo translations are not committed to the repository.
# Requires on the builder: git, make, sassc, gettext (msgfmt), glib-compile-schemas.
#
# For development builds set PULSAR_DOCK_LOCAL_DIR to a local checkout of the
# dash-to-dock repo to skip the GitHub clone:
#   PULSAR_DOCK_LOCAL_DIR=../dash-to-dock ./package-and-deploy.sh pulsaros-gnome
# ==============================================================================
echo "🚢 [ES] Instalando Pulsar Dock (fork propio de Dash to Dock)..."
echo "🚢 [EN] Installing Pulsar Dock (our own Dash to Dock fork)..."

PULSAR_DOCK_UUID="pulsar-dock@inled.es"
PULSAR_DOCK_TEMP="/tmp/pulsaros-pulsar-dock"
rm -rf "$PULSAR_DOCK_TEMP"

SCRIPT_DIR="$(dirname "$(realpath "$0")")"
if [ -n "$PULSAR_DOCK_LOCAL_DIR" ] && [ -d "$PULSAR_DOCK_LOCAL_DIR/.git" ]; then
    PULSAR_DOCK_SRC="$(realpath "$PULSAR_DOCK_LOCAL_DIR")"
    echo "📦 [ES] Usando copia local de Pulsar Dock: $PULSAR_DOCK_SRC"
    echo "📦 [EN] Using local Pulsar Dock checkout: $PULSAR_DOCK_SRC"
elif [ -d "$SCRIPT_DIR/../../dash-to-dock/.git" ]; then
    PULSAR_DOCK_SRC="$(realpath "$SCRIPT_DIR/../../dash-to-dock")"
    echo "📦 [ES] Usando copia local de Pulsar Dock: $PULSAR_DOCK_SRC"
    echo "📦 [EN] Using local Pulsar Dock checkout: $PULSAR_DOCK_SRC"
elif [ -d "$SCRIPT_DIR/../dash-to-dock/.git" ]; then
    PULSAR_DOCK_SRC="$(realpath "$SCRIPT_DIR/../dash-to-dock")"
    echo "📦 [ES] Usando copia local de Pulsar Dock: $PULSAR_DOCK_SRC"
    echo "📦 [EN] Using local Pulsar Dock checkout: $PULSAR_DOCK_SRC"
else
    # We use HTTPS URL to ensure compatibility without SSH keys in builder / chroot environments
    if git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "https://github.com/Inled-Pulsar-OS/dash-to-dock.git" "$PULSAR_DOCK_TEMP"; then
        PULSAR_DOCK_SRC="$PULSAR_DOCK_TEMP"
    else
        echo "❌ [ES] Error al clonar Pulsar Dock de GitHub."
        echo "❌ [EN] Error cloning Pulsar Dock from GitHub."
        exit 1
    fi
fi

if [ -d "$PULSAR_DOCK_SRC/_build" ]; then
    rm -rf "$PULSAR_DOCK_SRC/_build" 2>/dev/null || rm -rf "$PULSAR_DOCK_SRC/_build" || true
fi

PULSAR_DOCK_BUILD_LOG=$(mktemp)
if ! (cd "$PULSAR_DOCK_SRC" && make _build >"$PULSAR_DOCK_BUILD_LOG" 2>&1); then
    echo "❌ [ES] Fallo al compilar Pulsar Dock. Asegúrate de tener make, sassc, msgfmt y glib-compile-schemas en el entorno de build."
    echo "❌ [EN] Failed to build Pulsar Dock. Make sure make, sassc, msgfmt and glib-compile-schemas are available in the build environment."
    echo "--- Build Log ---"
    cat "$PULSAR_DOCK_BUILD_LOG"
    rm -f "$PULSAR_DOCK_BUILD_LOG"
    exit 1
fi
rm -f "$PULSAR_DOCK_BUILD_LOG"

PULSAR_DOCK_DEST="$STAGE_DIR/usr/share/gnome-shell/extensions/${PULSAR_DOCK_UUID}"
mkdir -p "$PULSAR_DOCK_DEST"
cp -r "$PULSAR_DOCK_SRC"/_build/* "$PULSAR_DOCK_DEST"/

# Preserve GNOME 51 compatibility patches for pulsar-dock
for _candidate in \
    "$SCRIPT_DIR/usr/share/gnome-shell/extensions/${PULSAR_DOCK_UUID}" \
    "/home/jaime/Documentos/pulsar/PKG/pulsaros-gnome/usr/share/gnome-shell/extensions/${PULSAR_DOCK_UUID}"; do
    if [ -f "$_candidate/docking.js" ]; then
        echo "🔧 [ES] Aplicando archivos de compatibilidad GNOME 51 para Pulsar Dock desde $_candidate..."
        cp -f "$_candidate"/dependencies/shell/ui.js "$PULSAR_DOCK_DEST/dependencies/shell/ui.js" 2>/dev/null || true
        cp -f "$_candidate"/dash.js "$PULSAR_DOCK_DEST/dash.js" 2>/dev/null || true
        cp -f "$_candidate"/docking.js "$PULSAR_DOCK_DEST/docking.js" 2>/dev/null || true
        cp -f "$_candidate"/utils.js "$PULSAR_DOCK_DEST/utils.js" 2>/dev/null || true
        cp -f "$_candidate"/windowPreview.js "$PULSAR_DOCK_DEST/windowPreview.js" 2>/dev/null || true
        break
    fi
done

# Ship the dock translations system-wide so the dashtodock gettext domain resolves
# Publicar las traducciones del dock en el sistema para que el dominio gettext dashtodock resuelva
if [ -d "$PULSAR_DOCK_SRC/_build/locale" ]; then
    for mo_file in "$PULSAR_DOCK_SRC"/_build/locale/*/LC_MESSAGES/*.mo; do
        [ -f "$mo_file" ] || continue
        lang_dir="$(basename "$(dirname "$(dirname "$mo_file")")")"
        mkdir -p "$STAGE_DIR/usr/share/locale/$lang_dir/LC_MESSAGES"
        cp "$mo_file" "$STAGE_DIR/usr/share/locale/$lang_dir/LC_MESSAGES/"
    done
fi

rm -rf "$PULSAR_DOCK_TEMP"
echo "✅ Extensión $PULSAR_DOCK_UUID lista."

# ==============================================================================
# PATCH WIGGLE EXTENSION FOR GNOME 50 / CLUTTER ANIMATION COMPATIBILITY
# PARCHEAR EXTENSIÓN WIGGLE PARA COMPATIBILIDAD CON GNOME 50 / CLUTTER
# ==============================================================================
WIGGLE_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/wiggle@mechtifs"
if [ -d "$WIGGLE_DIR" ]; then
    echo "🔧 [ES] Aplicando parche de estabilidad para Wiggle en GNOME 50..."
    echo "🔧 [EN] Applying stability patch for Wiggle on GNOME 50..."

    cat <<'WIGGLE_CURSOR_JS' > "$WIGGLE_DIR/cursor.js"
'use strict';

export default class Cursor {
    constructor() {
        this._tracker = global.backend.get_cursor_tracker(global.display);
    }

    get hot() {
        try {
            return this._tracker.get_hot();
        } catch (e) {
            return [0, 0];
        }
    }

    get sprite() {
        try {
            return this._tracker.get_sprite();
        } catch (e) {
            return null;
        }
    }

    show() {
        try {
            this._tracker.set_pointer_visible(true);
        } catch (e) {}
    }

    hide() {
        try {
            this._tracker.set_pointer_visible(false);
        } catch (e) {}
    }
}
WIGGLE_CURSOR_JS

    cat <<'WIGGLE_EFFECT_JS' > "$WIGGLE_DIR/effect.js"
'use strict';

import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Graphene from 'gi://Graphene';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import Cursor from './cursor.js';

export default class Effect extends St.Icon {
    static {
        GObject.registerClass(this);
    }

    constructor() {
        super();
        this.isHidden = true;
        this.magnifyDuration = 250;
        this.unmagnifyDuration = 150;
        this.unmagnifyDelay = 0;
        this.isWiggling = false;
        this.cursor = new Cursor();
        [this._hotX, this._hotY] = this.cursor.hot;
        this._spriteSize = this.cursor.sprite ? this.cursor.sprite.get_width() : 24;

        this._pivot = new Graphene.Point({
            x: this._hotX / this._spriteSize,
            y: this._hotY / this._spriteSize,
        });
    }

    set cursorSize(size) {
        this.icon_size = size;
        this._ratio = size / this._spriteSize;
    }

    set cursorPath(path) {
        this.gicon = Gio.Icon.new_for_string(path || GLib.path_get_dirname(import.meta.url.slice(7)) + '/icons/cursor.svg');
    }

    move(x, y) {
        this.set_position(x - this._hotX * this._ratio, y - this._hotY * this._ratio);
    }

    magnify() {
        if (this._unmagnifyDelayId) {
            GLib.Source.remove(this._unmagnifyDelayId);
            this._unmagnifyDelayId = null;
        }
        this._isInTransition = false;
        this.isWiggling = true;
        if (!this.get_parent()) {
            Main.uiGroup.add_child(this);
        }
        if (this.isHidden) {
            this.cursor.hide();
        }
        this.remove_all_transitions();
        this.ease({
            duration: this.magnifyDuration,
            mode: Clutter.AnimationMode.EASE_IN_QUAD,
            scale_x: 1.0,
            scale_y: 1.0,
            pivot_point: this._pivot,
        });
    }

    unmagnify() {
        if (this._isInTransition) {
            return;
        }
        this._isInTransition = true;
        if (this._unmagnifyDelayId) {
            GLib.Source.remove(this._unmagnifyDelayId);
        }
        this._unmagnifyDelayId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, this.unmagnifyDelay, () => {
            this._unmagnifyDelayId = null;
            this.remove_all_transitions();
            this.ease({
                duration: this.unmagnifyDuration,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                scale_x: 1.0 / this._ratio,
                scale_y: 1.0 / this._ratio,
                pivot_point: this._pivot,
                onComplete: () => {
                    this._cleanup();
                },
                onStopped: () => {
                    this._cleanup();
                }
            });
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, this.unmagnifyDuration + 50, () => {
                this._cleanup();
                return GLib.SOURCE_REMOVE;
            });
            return GLib.SOURCE_REMOVE;
        });
    }

    _cleanup() {
        if (this.get_parent()) {
            Main.uiGroup.remove_child(this);
        }
        this.cursor.show();
        this.isWiggling = false;
        this._isInTransition = false;
    }

    destroy() {
        if (this._unmagnifyDelayId) {
            GLib.Source.remove(this._unmagnifyDelayId);
            this._unmagnifyDelayId = null;
        }
        this._cleanup();
    }
}
WIGGLE_EFFECT_JS

    # GNOME 51 removed the bundled shell module
    # "resource:///org/gnome/shell/ui/pointerWatcher.js", so we ship our own
    # copy alongside the extension and import it relatively.
    # GNOME 51 eliminó el módulo "resource:///org/gnome/shell/ui/pointerWatcher.js"
    # de la shell, así que incluimos nuestra propia copia junto a la extensión
    # y la importamos de forma relativa.
    cat <<'WIGGLE_POINTER_WATCHER_JS' > "$WIGGLE_DIR/pointerWatcher.js"
import GLib from 'gi://GLib';

class PointerWatch {
    constructor(watcher, interval, callback) {
        this.watcher = watcher;
        this.interval = interval;
        this.callback = callback;
        this.paused = false;
    }

    remove() {
        this.watcher._removeWatch(this);
    }
}

class PointerWatcher {
    constructor() {
        this._watches = [];
        this._idleId = 0;
        this._lastX = -1;
        this._lastY = -1;
    }

    addWatch(interval, callback) {
        const watch = new PointerWatch(this, interval, callback);
        this._watches.push(watch);
        this._updateTimer();
        return watch;
    }

    _removeWatch(watch) {
        const idx = this._watches.indexOf(watch);
        if (idx !== -1) {
            this._watches.splice(idx, 1);
            this._updateTimer();
        }
    }

    _updateTimer() {
        if (this._watches.length > 0 && !this._idleId) {
            this._idleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 40, () => {
                try {
                    const [x, y] = global.get_pointer();
                    if (x !== this._lastX || y !== this._lastY) {
                        this._lastX = x;
                        this._lastY = y;
                        for (const watch of this._watches) {
                            if (!watch.paused) {
                                try {
                                    watch.callback(x, y);
                                } catch (e) {
                                    console.error('[PointerWatcher] callback error:', e);
                                }
                            }
                        }
                    }
                } catch (e) {}
                return GLib.SOURCE_CONTINUE;
            });
        } else if (this._watches.length === 0 && this._idleId) {
            GLib.source_remove(this._idleId);
            this._idleId = 0;
        }
    }
}

let _pointerWatcherInstance = null;
export function getPointerWatcher() {
    if (!_pointerWatcherInstance)
        _pointerWatcherInstance = new PointerWatcher();
    return _pointerWatcherInstance;
}
WIGGLE_POINTER_WATCHER_JS

    cat <<'WIGGLE_EXT_JS' > "$WIGGLE_DIR/extension.js"
'use strict';

import GLib from 'gi://GLib';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import { getPointerWatcher } from './pointerWatcher.js';

import { Field } from './const.js';
import Effect from './effect.js';
import History from './history.js';

const initSettings = (settings, entries) => {
    const getValue = (name, type) => ({
        'b': () => settings.get_boolean(name),
        'd': () => settings.get_double(name),
        'i': () => settings.get_int(name),
        's': () => settings.get_string(name),
    }[type]());
    entries.forEach(([name, type, func]) => {
        func(getValue(name, type));
        settings.connect(`changed::${name}`, () => func(getValue(name, type)));
    });
};

export default class WiggleExtension extends Extension {
    _onCheckIntervalChange(interval) {
        if (this._checkTimeoutId) {
            GLib.Source.remove(this._checkTimeoutId);
            this._checkTimeoutId = null;
        }
    }

    _onDrawIntervalChange(interval) {
        this._drawInterval = interval;
        if (this._drawIntervalWatch && this._pointerWatcher) {
            this._pointerWatcher._removeWatch(this._drawIntervalWatch);
        }
        if (this._pointerWatcher) {
            this._drawIntervalWatch = this._pointerWatcher.addWatch(interval, (x, y) => {
                if (this._history) {
                    this._history.push(x, y);
                    if (this._history.check()) {
                        if (this._effect && !this._effect.isWiggling) {
                            this._effect.move(x, y);
                            this._effect.magnify();
                        }
                    } else if (this._effect && this._effect.isWiggling) {
                        this._effect.unmagnify();
                    }
                }
                if (this._effect && this._effect.isWiggling) {
                    this._effect.move(x, y);
                }
            });
        }
    }

    _togglePointerWatcher(state) {
        if (state) {
            if (!this._drawIntervalWatch) {
                this._onDrawIntervalChange(this._drawInterval);
            }
        } else {
            if (this._effect && this._effect.isWiggling) {
                this._effect.unmagnify();
            }
            if (this._drawIntervalWatch && this._pointerWatcher) {
                this._pointerWatcher._removeWatch(this._drawIntervalWatch);
                this._drawIntervalWatch = null;
            }
            if (this._history) {
                this._history.clear();
            }
        }
    }

    enable() {
        this._pointerWatcher = getPointerWatcher();
        this._history = new History();
        this._effect = new Effect();
        this._settings = this.getSettings();
        initSettings(this._settings, [
            [Field.HIDE, 'b', (r) => {if (this._effect) this._effect.isHidden = r}],
            [Field.SIZE, 'i', (r) => {if (this._effect) this._effect.cursorSize = r}],
            [Field.PATH, 's', (r) => {if (this._effect) this._effect.cursorPath = r}],
            [Field.MAGN, 'i', (r) => {if (this._effect) this._effect.magnifyDuration = r}],
            [Field.UMGN, 'i', (r) => {if (this._effect) this._effect.unmagnifyDuration = r}],
            [Field.DLAY, 'i', (r) => {if (this._effect) this._effect.unmagnifyDelay = r}],

            [Field.SAMP, 'i', (r) => {if (this._history) this._history.sampleSize = r}],
            [Field.RADI, 'i', (r) => {if (this._history) this._history.radiansThreshold = r}],
            [Field.DIST, 'i', (r) => {if (this._history) this._history.distanceThreshold = r}],
            [Field.CHCK, 'i', (r) => this._onCheckIntervalChange(r)],
            [Field.DRAW, 'i', (r) => this._onDrawIntervalChange(r)],
        ]);
    }

    disable() {
        if (this._checkTimeoutId) {
            GLib.Source.remove(this._checkTimeoutId);
            this._checkTimeoutId = null;
        }
        this._togglePointerWatcher(false);
        if (this._effect) {
            this._effect.destroy();
            this._effect = null;
        }
        this._pointerWatcher = null;
        this._history = null;
        this._settings = null;
    }
}
WIGGLE_EXT_JS
fi

# ==============================================================================
# DOWNLOAD AND INSTALL LIQUID GLASS EXTENSION FROM GITHUB
# DESCARGAR E INSTALAR LA EXTENSIÓN LIQUID GLASS DESDE GITHUB
# ==============================================================================
echo "🎨 [ES] Descargando extensión Liquid Glass desde GitHub..."
echo "🎨 [EN] Downloading Liquid Glass extension from GitHub..."
TEMP_LG="/tmp/pulsaros-liquid-glass"
rm -rf "$TEMP_LG"
# We use HTTPS URL to ensure compatibility without SSH keys in builder / chroot environments
# Usamos la URL HTTPS para asegurar la compatibilidad sin claves SSH en entornos de compilación / chroot
if git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "https://github.com/ryohsuke1231/liquid-glass.git" "$TEMP_LG"; then
    echo "📦 [ES] Instalando Liquid Glass en el staging..."
    echo "📦 [EN] Installing Liquid Glass to staging..."
    mkdir -p "$STAGE_DIR/usr/share/gnome-shell/extensions"
    cp -r "$TEMP_LG/liquid-glass@thinkingcoding1231.gmail.com" "$STAGE_DIR/usr/share/gnome-shell/extensions/"
    rm -rf "$TEMP_LG"

    # Declarative patches for Liquid Glass:
    # 1. Set refined default corner radius (14px) and expand (4px) in schema
    find "$STAGE_DIR/usr/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com" -name "*.gschema.xml" | while read -r schema_file; do
        python3 -c "
import sys, re
f = sys.argv[1]
with open(f, 'r') as fp: c = fp.read()
c = re.sub(r'(<key name=\"panel-menu-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>14.0\g<2>', c)
c = re.sub(r'(<key name=\"panel-menu-glass-expand\" type=\"i\">\s*<default>)[^<]+(</default>)', r'\g<1>4\g<2>', c)
c = re.sub(r'(<key name=\"menu-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>14.0\g<2>', c)
c = re.sub(r'(<key name=\"menu-glass-expand\" type=\"i\">\s*<default>)[^<]+(</default>)', r'\g<1>4\g<2>', c)
c = re.sub(r'(<key name=\"desktop-menu-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>14.0\g<2>', c)
c = re.sub(r'(<key name=\"notification-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>16.0\g<2>', c)
c = re.sub(r'(<key name=\"osd-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>16.0\g<2>', c)
c = re.sub(r'(<key name=\"quick-settings-corner-radius\" type=\"d\">\s*<default>)[^<]+(</default>)', r'\g<1>18.0\g<2>', c)
with open(f, 'w') as fp: fp.write(c)
" "$schema_file"
    done

    # 2. Exclude Sayri and Pulsar OS Welcome from Liquid Glass window shader
    LG_APP_MGR="$STAGE_DIR/usr/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com/dist/applicationManager.js"
    if [ -f "$LG_APP_MGR" ]; then
        sed -i 's/_shouldApplyToWindow(windowActor) {/_shouldApplyToWindow(windowActor) {\n        const _mWin = windowActor.get_meta_window();\n        if (_mWin) {\n            const _wm = (_mWin.get_wm_class() || "").toLowerCase();\n            const _tt = (_mWin.get_title() || "").toLowerCase();\n            if (_wm.includes("sayri") || _tt.includes("sayri") || _wm.includes("welcome") || _tt.includes("welcome")) return false;\n        }/g' "$LG_APP_MGR"
    fi

    # 3. Add DialogManager to Liquid Glass for modal/system dialogs (Auth, Run, Power off, etc.)
    LG_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/liquid-glass@thinkingcoding1231.gmail.com"
    cat << 'EOF' > "$LG_DIR/dist/dialogManager.js"
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import Clutter from 'gi://Clutter';
import St from 'gi://St';
import GLib from 'gi://GLib';
import { BackdropGlass } from './rendering/backdropGlass.js';
import { hexToColorArray } from './animation/colors.js';

const SHADER_PADDING = 20;

export class DialogManager {
    constructor(extensionPath, settings, logger) {
        this.extensionPath = extensionPath;
        this._settings = settings;
        this._logger = logger;
        this._dialogStates = new Map();
        this._signals = [];
        this._enabled = false;
    }

    setup() {
        if (!this._settings) return;
        this._enabled = true;
        this._bindSignals();
        this._scanExisting();
    }

    _bindSignals() {
        const group = Main.layoutManager.modalDialogGroup;
        if (group) {
            this._signals.push({
                obj: group,
                id: group.connect('child-added', (_g, child) => this._onChildAdded(child)),
            });
            this._signals.push({
                obj: group,
                id: group.connect('child-removed', (_g, child) => this._onChildRemoved(child)),
            });
        }
    }

    _scanExisting() {
        const group = Main.layoutManager.modalDialogGroup;
        if (!group) return;
        for (const child of group.get_children()) {
            this._onChildAdded(child);
        }
    }

    _findDialogBox(actor) {
        if (!actor) return null;
        if (actor.has_style_class_name?.('modal-dialog') || actor.has_style_class_name?.('run-dialog'))
            return actor;
        if (actor.get_children) {
            for (const c of actor.get_children()) {
                const found = this._findDialogBox(c);
                if (found) return found;
            }
        }
        return null;
    }

    _onChildAdded(child) {
        if (!this._enabled || !child) return;
        // Skip our own backdrops: they live in the same modalDialogGroup, so
        // without this the group's child-added signal would try to glass the
        // glass and spawn an endless cascade.
        if (child._pulsarDialogGlass) return;
        GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            if (!this._enabled || !child || !child.get_parent()) return GLib.SOURCE_REMOVE;
            this._attachGlass(child);
            return GLib.SOURCE_REMOVE;
        });
    }

    _onChildRemoved(child) {
        this._detachGlass(child);
    }

    _attachGlass(child) {
        if (!child || child._pulsarDialogGlass) return;
        if (this._dialogStates.has(child)) return;
        // Only glass real dialogs. Falling back to `child` used to dress every
        // unrelated actor dropped in the modalDialogGroup (including our own
        // backdrops) with glass, feeding the cascade.
        const targetBox = this._findDialogBox(child);
        if (!targetBox) return;
        if (typeof targetBox.add_style_class_name === 'function')
            targetBox.add_style_class_name('liquid-glass-transparent');

        const glass = new BackdropGlass({
            extensionPath: this.extensionPath,
            settings: this._settings,
            logger: this._logger,
            owner: 'dialog',
        });
        glass.set_size(1.0, 1.0);
        glass.set_pivot_point(0.0, 0.0);

        // Tag the backdrop *before* it enters the group so the child-added
        // handler above can recognise and ignore it.
        glass._pulsarDialogGlass = true;
        const parent = child.get_parent() || Main.layoutManager.modalDialogGroup;
        parent.insert_child_below(glass, child);

        const cornerRadius = 20.0;
        let tintColorStr = '#000000';
        try { tintColorStr = this._settings.get_string('menu-tint-color') || '#000000'; } catch (e) {}
        let tintStrength = 0.12;
        try { tintStrength = this._settings.get_double('menu-tint-strength') || 0.12; } catch (e) {}
        let blurRadius = 16;
        try { blurRadius = this._settings.get_int('menu-blur-radius') || 16; } catch (e) {}

        glass.setPadding(SHADER_PADDING);
        glass.setTintColor(...hexToColorArray(tintColorStr));
        glass.setTintStrength(tintStrength);
        glass.setCornerRadius(cornerRadius);
        glass.setIsDock(false);
        glass.setBlurRadius(blurRadius);

        const syncGeometry = () => {
            if (!child || !targetBox || !glass) return;
            if (!child.visible || targetBox.opacity === 0) {
                glass.hide();
                return;
            }
            try {
                const [x, y] = targetBox.get_transformed_position();
                const [w, h] = targetBox.get_transformed_size();
                if (w > 0 && h > 0) {
                    glass.set_position(x - SHADER_PADDING, y - SHADER_PADDING);
                    glass.set_size(w + SHADER_PADDING * 2, h + SHADER_PADDING * 2);
                    glass.setCornerRadius(cornerRadius);
                    glass.show();
                    glass.opacity = targetBox.get_paint_opacity ? targetBox.get_paint_opacity() : targetBox.opacity;
                }
            } catch (e) {}
        };

        const conns = [
            child.connect('notify::allocation', syncGeometry),
            child.connect('notify::visible', syncGeometry),
            child.connect('notify::opacity', syncGeometry),
            targetBox.connect('notify::allocation', syncGeometry),
            targetBox.connect('notify::visible', syncGeometry),
            targetBox.connect('notify::opacity', syncGeometry),
        ];
        // When the dialog is destroyed the child (and a descendant targetBox)
        // are already disposed: their signal handlers die with them, so undo
        // nothing but our own backdrop here. Touching the disposed actor is
        // what produced the "already disposed" warnings.
        const destroyId = child.connect('destroy', () => {
            this._dialogStates.delete(child);
            try { glass.destroy(); } catch (e) {}
        });

        this._dialogStates.set(child, { targetBox, glass, conns, destroyId });
        syncGeometry();
    }

    _detachGlass(child) {
        const state = this._dialogStates.get(child);
        if (!state) return;
        this._dialogStates.delete(child);

        for (const id of state.conns) {
            try { child.disconnect(id); } catch (e) {}
            try { state.targetBox.disconnect(id); } catch (e) {}
        }
        if (state.destroyId) {
            try { child.disconnect(state.destroyId); } catch (e) {}
        }
        try { state.targetBox.remove_style_class_name('liquid-glass-transparent'); } catch (e) {}
        try { state.glass.destroy(); } catch (e) {}
    }

    destroy() {
        this._enabled = false;
        for (const s of this._signals) {
            try { s.obj.disconnect(s.id); } catch (e) {}
        }
        this._signals = [];
        for (const child of [...this._dialogStates.keys()]) {
            this._detachGlass(child);
        }
    }
}
EOF

    # Register DialogManager in extension.js
    sed -i "s/import {ApplicationManager} from '.\/dist\/applicationManager.js';/import {ApplicationManager} from '.\/dist\/applicationManager.js';\nimport {DialogManager} from '.\/dist\/dialogManager.js';/g" "$LG_DIR/extension.js"
    sed -i "s/this._applicationManager.setup();/this._applicationManager.setup();\n    this._dialogManager = new DialogManager(path, this._settings, this._logger);\n    this._dialogManager.setup();/g" "$LG_DIR/extension.js"
    sed -i "s/this._applicationManager = null;/this._applicationManager = null;\n    if (this._dialogManager) { this._dialogManager.destroy(); this._dialogManager = null; }/g" "$LG_DIR/extension.js"

    # 3b. Stop the Quick Settings colour timer from touching disposed actors.
    #     Liquid Glass styles every toggle's sub-actors (label/icon boxes), but
    #     GNOME Shell rebuilds those sub-actors on each state change. A disposed
    #     GObject accessed from GJS logs "has been already disposed" as a
    #     g_warning (NOT a catchable JS exception) on every 100ms sample tick,
    #     flooding the journal. Prune each sub from styledSubs when it dies and
    #     only restyle sub-actors still inside the pod. Also used by the
    #     notification-center glass manager (same ToggleStyles class).
    LG_TOGGLE_STYLES="$LG_DIR/dist/quickSettings/toggleStyles.js"
    if [ -f "$LG_TOGGLE_STYLES" ]; then
        python3 - "$LG_TOGGLE_STYLES" << 'PYEOF'
import sys
path = sys.argv[1]
with open(path, 'r') as fp:
    c = fp.read()
if 'pod.contains(actor)' not in c:
    changes = [
        (
            """    for (const { actor, origStyle } of entry.styledSubs) {
        if (actor instanceof St.Widget)
            actor.set_style(origStyle || null);
    }
}""",
            """    for (const { actor, origStyle } of entry.styledSubs) {
        if (actor instanceof St.Widget && pod.contains(actor))
            actor.set_style(origStyle || null);
    }
}""",
        ),
        (
            """            const origStyle = sub.get_style() || '';
            sub.set_style(_withOverride(origStyle));
            if (!known.has(sub))
                entry.styledSubs.push({ actor: sub, origStyle });""",
            """            const origStyle = sub.get_style() || '';
            sub.set_style(_withOverride(origStyle));
            if (!known.has(sub)) {
                const record = { actor: sub, origStyle, destroyId: 0 };
                // Quick Settings rebuilds its toggle sub-actors on every state
                // change; drop each sub as soon as it is destroyed so the
                // colour timer never touches a disposed actor.
                record.destroyId = sub.connect('destroy', () => {
                    const idx = entry.styledSubs.indexOf(record);
                    if (idx >= 0)
                        entry.styledSubs.splice(idx, 1);
                });
                entry.styledSubs.push(record);
            }""",
        ),
        (
            """        for (const { actor, origStyle } of entry.styledSubs) {
            if (actor instanceof St.Widget)
                actor.set_style(origStyle || null);
        }
        if (this._releaseIfSliderLostPill(pod, entry))""",
            """        for (const { actor, origStyle } of entry.styledSubs) {
            if (actor instanceof St.Widget && pod.contains(actor))
                actor.set_style(origStyle || null);
        }
        if (this._releaseIfSliderLostPill(pod, entry))""",
        ),
        (
            """        for (const { actor, origStyle } of entry.styledSubs) {
            if (actor instanceof St.Widget)
                actor.set_style(_withOverride(origStyle));
        }
    }""",
            """        for (const { actor, origStyle } of entry.styledSubs) {
            if (actor instanceof St.Widget && pod.contains(actor))
                actor.set_style(_withOverride(origStyle));
        }
    }""",
        ),
    ]
    ok = True
    for old, new in changes:
        if c.count(old) != 1:
            ok = False
            break
        c = c.replace(old, new, 1)
    if ok:
        with open(path, 'w') as fp:
            fp.write(c)
        print("patched toggleStyles.js")
    else:
        print("WARN: toggleStyles.js shape changed, skipping patch")
PYEOF
    fi

    # Add dialog stylesheet rules
    cat << 'EOF' >> "$LG_DIR/stylesheet.css"

/* Dialogs with Liquid Glass */
.modal-dialog.liquid-glass-transparent,
.run-dialog.liquid-glass-transparent {
    background-color: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.20) !important;
    border-radius: 20px !important;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45) !important;
}
EOF

    # 4. Add NotificationCenterGlassManager: per-card liquid glass in the
    #    notification center (dateMenu), reusing Quick Settings toggles mode.
    cat << 'NCJS' > "$LG_DIR/dist/notificationCenterGlass.js"
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { QuickSettingsManager } from './quickSettingsManager.js';
import { ToggleStyles } from './quickSettings/toggleStyles.js';
import { hexToColorArray } from './animation/colors.js';

export class NotificationCenterGlassManager extends QuickSettingsManager {
    constructor(extensionPath, settings, logger) {
        super(extensionPath, settings, logger);
        const menu = Main.panel.statusArea.dateMenu?.menu ?? null;
        this.menu = menu;
        this.targetActor = menu?.actor ?? null;
        this.animActor = menu?.box ?? null;
        this._toggleStyles = new ToggleStyles(logger, () => !!this.menu?.isOpen);
        this._applyTo = 'toggles';
        this._activeMode = 'toggles';
    }

    _togglesDesired() {
        return !!this._settings &&
            this._settings.get_boolean('enable-quick-settings-glass') &&
            this._settings.get_int('quick-settings-apply-to') === 1;
    }

    setup() {
        if (!this._settings)
            return;
        this._bindSettings();
        this._applyTo = 'toggles';
        this._toggleBaseStrength = this._settings.get_double('quick-settings-toggle-tint-strength');
        this._toggleCornerRadius = this._settings.get_double('quick-settings-toggle-corner-radius');
        if (this._togglesDesired())
            this._applyEffect();
    }

    _bindSettings() {
        const connectSetting = (key, callback) => {
            this._settingsSignals.push(this._settings.connect(`changed::${key}`, callback.bind(this)));
        };
        connectSetting('enable-quick-settings-glass', () => this._syncActivation());
        connectSetting('quick-settings-apply-to', () => this._syncActivation());
        connectSetting('quick-settings-tint-color', () => {
            this._tintColorArray = hexToColorArray(this._settings.get_string('quick-settings-tint-color'));
            if (this.glass)
                this.glass.setTintColor(...this._tintColorArray);
        });
        connectSetting('quick-settings-tint-strength', () => {
            if (this.glass)
                this.glass.setTintStrength(this._settings.get_double('quick-settings-tint-strength'));
        });
        connectSetting('quick-settings-blur-radius', () => {
            if (this.glass)
                this.glass.setBlurRadius(this._settings.get_int('quick-settings-blur-radius'));
        });
        connectSetting('quick-settings-toggle-tint-strength', () => {
            this._toggleBaseStrength = this._settings.get_double('quick-settings-toggle-tint-strength');
        });
        connectSetting('quick-settings-toggle-corner-radius', () => {
            this._toggleCornerRadius = this._settings.get_double('quick-settings-toggle-corner-radius');
            if (this.glass && this._activeMode === 'toggles')
                this.glass.setCornerRadius(this._toggleCornerRadius);
        });
        connectSetting('quick-settings-brightness', () => {
            if (this.glass)
                this.glass.setBrightness(this._settings.get_double('quick-settings-brightness'));
        });
        connectSetting('quick-settings-saturation', () => {
            if (this.glass)
                this.glass.setSaturation(this._settings.get_double('quick-settings-saturation'));
        });
        connectSetting('quick-settings-contrast', () => {
            if (this.glass)
                this.glass.setContrast(this._settings.get_double('quick-settings-contrast'));
        });
    }

    _syncActivation() {
        if (this._togglesDesired()) {
            if (!this._isEffectActive)
                this._applyEffect();
        } else if (this._isEffectActive) {
            this._removeEffect();
        }
    }

    _applyEffect() {
        if (this._isEffectActive || !this._togglesDesired() || !this.targetActor)
            return;
        this._isEffectActive = true;
        this._activeMode = 'toggles';
        this._applyTo = 'toggles';
        this._tintColorArray = hexToColorArray(this._settings.get_string('quick-settings-tint-color'));
        this._applyToggleEffect();
    }

    _restoreMenuActors() {
    }

    _applyMenuOffsets() {
    }
}
NCJS

    # Register NotificationCenterGlassManager in extension.js (using QS anchors)
    sed -i "s/import {QuickSettingsManager} from '.\/dist\/quickSettingsManager.js';/import {QuickSettingsManager} from '.\/dist\/quickSettingsManager.js';\nimport {NotificationCenterGlassManager} from '.\/dist\/notificationCenterGlass.js';/g" "$LG_DIR/extension.js"
    sed -i "s/      this._quickSettingsManager.setup();/      this._quickSettingsManager.setup();\n      this._notificationCenterGlass = new NotificationCenterGlassManager(path, this._settings, this._logger);\n      this._notificationCenterGlass.setup();/g" "$LG_DIR/extension.js"
    sed -i "s/this._quickSettingsManager?.cleanup();/this._quickSettingsManager?.cleanup();\n      this._notificationCenterGlass?.cleanup();\n      this._notificationCenterGlass = null;/g" "$LG_DIR/extension.js"
    sed -i "s/this._uiManager.setup();/\/\/ this._uiManager.setup();/g" "$LG_DIR/extension.js"
else
    echo "❌ [ES] Error al clonar Liquid Glass de GitHub."
    echo "❌ [EN] Error cloning Liquid Glass from GitHub."
    exit 1
fi


# Copy all extensions' .gschema.xml files to the global schemas directory so gsettings and dconf can manage them
# Copiar todos los archivos .gschema.xml de las extensiones al directorio global de esquemas para que gsettings y dconf puedan gestionarlos
echo "⚙️ [ES] Copiando esquemas xml de extensiones al directorio global..."
echo "⚙️ [EN] Copying extensions' xml schemas to the global schemas directory..."
mkdir -p "$STAGE_DIR/usr/share/glib-2.0/schemas"
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -name "*.gschema.xml" -exec cp {} "$STAGE_DIR/usr/share/glib-2.0/schemas/" \; 2>/dev/null || true

# Compilar esquemas locales de las extensiones dentro de staging para que estén listos
echo "Compilando esquemas locales de extensiones..."
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -name schemas -type d 2>/dev/null | while read -r schema_path; do
    glib-compile-schemas "$schema_path" || true
done

# Compilar también el directorio global de schemas en staging
echo "Compilando esquemas globales en staging..."
glib-compile-schemas "$STAGE_DIR/usr/share/glib-2.0/schemas" || true

# ==============================================================================
# KIWI MENU LOGO REPLACEMENT
# ==============================================================================
# English: Replace the copyrighted Apple logo in Kiwi Menu with the Pulsar OS logo
# Español: Reemplazar el logo de Apple con copyright en Kiwi Menu con el logo de Pulsar OS


# Ensure correct permissions for all files and folders under extensions and schemas directory (avoiding permission denied in Gnome Shell)
# Asegurar permisos correctos para todos los archivos y carpetas bajo el directorio de extensiones y esquemas (evitando fallos de permisos en Gnome Shell)
echo "⚙️ [ES] Asegurando permisos de lectura y ejecución para las extensiones..."
echo "⚙️ [EN] Ensuring read and execute permissions for the extensions..."
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -type d -exec chmod 755 {} \; 2>/dev/null || true
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -type f -exec chmod 644 {} \; 2>/dev/null || true
find "$STAGE_DIR/usr/share/glib-2.0/schemas" -type f -exec chmod 644 {} \; 2>/dev/null || true

# DING extension needs its background process script to be executable
# La extensión DING necesita que su script de proceso en segundo plano sea ejecutable
if [ -f "$STAGE_DIR/usr/share/gnome-shell/extensions/ding@rastersoft.com/app/ding.js" ]; then
    chmod 755 "$STAGE_DIR/usr/share/gnome-shell/extensions/ding@rastersoft.com/app/ding.js"
fi

# GSConnect config.js path fix: EGO builds may ship with /usr/local/share prefixes
# but we install to /usr/share, so patch the paths to match
# Parche de rutas en config.js de GSConnect: las builds de EGO pueden traer prefijos /usr/local/share
# pero instalamos en /usr/share, así que ajustamos las rutas
GSCONNECT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/gsconnect@andyholmes.github.io"
GSCONNECT_CONFIG="$GSCONNECT_DIR/config.js"
if [ -f "$GSCONNECT_CONFIG" ]; then
    sed -i "s|'/usr/local/share/gnome-shell/extensions/gsconnect@andyholmes.github.io'|'/usr/share/gnome-shell/extensions/gsconnect@andyholmes.github.io'|g" "$GSCONNECT_CONFIG"
    sed -i "s|'/usr/local/share/glib-2.0/schemas'|'/usr/share/glib-2.0/schemas'|g" "$GSCONNECT_CONFIG"
    sed -i "s|'/usr/local/share/locale'|'/usr/share/locale'|g" "$GSCONNECT_CONFIG"
    sed -i "s|'/usr/local/lib/x86_64-linux-gnu'|'/usr/lib'|g" "$GSCONNECT_CONFIG" 2>/dev/null || true
    sed -i "s|'/usr/lib/x86_64-linux-gnu'|'/usr/lib'|g" "$GSCONNECT_CONFIG" 2>/dev/null || true
    echo "✅ [ES] Rutas de config.js de GSConnect parcheadas a /usr/share"
    echo "✅ [EN] GSConnect config.js paths patched to /usr/share"
fi

# GSConnect D-Bus service, desktop files and icons (required for service activation & UI)
if [ -d "$GSCONNECT_DIR" ]; then
    echo "🔌 [ES] Instalando servicio D-Bus y archivos de integración para GSConnect..."
    echo "🔌 [EN] Installing D-Bus service and integration files for GSConnect..."

    mkdir -p "$STAGE_DIR/usr/share/dbus-1/services"
    cat << 'EOF' > "$STAGE_DIR/usr/share/dbus-1/services/org.gnome.Shell.Extensions.GSConnect.service"
[D-BUS Service]
Name=org.gnome.Shell.Extensions.GSConnect
Exec=/usr/share/gnome-shell/extensions/gsconnect@andyholmes.github.io/service/daemon.js
EOF
    chmod 644 "$STAGE_DIR/usr/share/dbus-1/services/org.gnome.Shell.Extensions.GSConnect.service"

    mkdir -p "$STAGE_DIR/usr/share/applications"
    cat << 'EOF' > "$STAGE_DIR/usr/share/applications/org.gnome.Shell.Extensions.GSConnect.desktop"
[Desktop Entry]
Type=Application
Name=GSConnect
Exec=gapplication launch org.gnome.Shell.Extensions.GSConnect %U
Terminal=false
NoDisplay=true
Icon=org.gnome.Shell.Extensions.GSConnect
Categories=Network;
MimeType=x-scheme-handler/sms;x-scheme-handler/tel;
DBusActivatable=true
X-GNOME-UsesNotifications=true
EOF
    chmod 644 "$STAGE_DIR/usr/share/applications/org.gnome.Shell.Extensions.GSConnect.desktop"

    cat << 'EOF' > "$STAGE_DIR/usr/share/applications/org.gnome.Shell.Extensions.GSConnect.Preferences.desktop"
[Desktop Entry]
Type=Application
Name=GSConnect Preferences
Exec=gapplication action org.gnome.Shell.Extensions.GSConnect preferences
Terminal=false
NoDisplay=true
Icon=org.gnome.Shell.Extensions.GSConnect
Categories=Network;
EOF
    chmod 644 "$STAGE_DIR/usr/share/applications/org.gnome.Shell.Extensions.GSConnect.Preferences.desktop"

    if [ -f "$GSCONNECT_DIR/org.gnome.Shell.Extensions.GSConnect.gresource" ] && command -v gresource >/dev/null 2>&1; then
        mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps" "$STAGE_DIR/usr/share/icons/hicolor/symbolic/apps"
        gresource extract "$GSCONNECT_DIR/org.gnome.Shell.Extensions.GSConnect.gresource" /org/gnome/Shell/Extensions/GSConnect/icons/org.gnome.Shell.Extensions.GSConnect.svg > "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/org.gnome.Shell.Extensions.GSConnect.svg" 2>/dev/null || true
        gresource extract "$GSCONNECT_DIR/org.gnome.Shell.Extensions.GSConnect.gresource" /org/gnome/Shell/Extensions/GSConnect/icons/org.gnome.Shell.Extensions.GSConnect-symbolic.svg > "$STAGE_DIR/usr/share/icons/hicolor/symbolic/apps/org.gnome.Shell.Extensions.GSConnect-symbolic.svg" 2>/dev/null || true
    fi

    # GSConnect service scripts permissions
    if [ -d "$GSCONNECT_DIR/service" ]; then
        find "$GSCONNECT_DIR/service" -name "*.js" -exec chmod 755 {} \; 2>/dev/null || true
    fi
    if [ -f "$GSCONNECT_DIR/gsconnect-preferences" ]; then
        chmod 755 "$GSCONNECT_DIR/gsconnect-preferences"
    fi
fi

# Ensure executable permissions for scripts
if [ -f "$STAGE_DIR/usr/bin/pulsaros-hide-overview" ]; then
    chmod 755 "$STAGE_DIR/usr/bin/pulsaros-hide-overview"
fi
if [ -f "$STAGE_DIR/usr/bin/pulsar-sync-accent" ]; then
    chmod 755 "$STAGE_DIR/usr/bin/pulsar-sync-accent"
fi

echo "✅ Proceso de extensiones finalizado."
